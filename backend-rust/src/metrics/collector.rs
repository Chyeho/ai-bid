//! MetricsCollector — 指标采集器。
//!
//! ## 使用方式
//!
//! ```ignore
//! let collector = Arc::new(Mutex::new(MetricsCollector::new(
//!     SCHEMA_VERSION,
//!     "qwen-plus",
//! )));
//!
//! // 记录阶段
//! collector.lock().await.record_stage(
//!     SemanticStage::Embedding,
//!     duration_ms,
//!     StageDetail::Embedding { chunk_count: 156, dimension: 1024 },
//! );
//!
//! // 记录 LLM 调用（从 react_loop 内调用）
//! collector.lock().await.record_llm_call(LlmCallRecord { ... });
//!
//! // 最终输出
//! let metrics = collector.lock().await.finalize(meta);
//! fs::write("output/runs/run_001.json", serde_json::to_string_pretty(&metrics)?)?;
//! ```

use crate::metrics::schema::*;
use std::collections::HashMap;
use std::time::Instant;

/// DashScope qwen-plus 定价（2025，CNY / 1M tokens）
const QWEN_PLUS_INPUT_PRICE: f64 = 0.8;
const QWEN_PLUS_OUTPUT_PRICE: f64 = 2.0;

/// DashScope qwen-turbo 定价（2025 官方）
const QWEN_TURBO_INPUT_PRICE: f64 = 0.5;
const QWEN_TURBO_OUTPUT_PRICE: f64 = 2.0;

/// 指标采集器。
///
/// 线程安全（`Arc<Mutex<>>`），可在多 Agent 并行场景中共享。
/// 采集原始数据，在 `finalize()` 时聚合为 `RunMetrics`。
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    schema_version: String,
    model_name: String,
    start_time: Instant,

    // Layer 1: 阶段记录
    stages: Vec<RawStageRecord>,

    // Layer 2: LLM 调用明细
    llm_calls: Vec<LlmCallRecord>,
    total_tokens_input: u64,
    total_tokens_output: u64,
    total_llm_duration_ms: u64,

    // Layer 3: Agent finding 统计 + 详情
    findings_detail: Vec<serde_json::Value>,
    agent_raw_findings: HashMap<String, usize>,
    agent_high_findings: HashMap<String, usize>,
    agent_medium_findings: HashMap<String, usize>,
    agent_low_findings: HashMap<String, usize>,
    agent_info_findings: HashMap<String, usize>,
    agent_confidence_sum: HashMap<String, f64>,
    agent_confidence_count: HashMap<String, usize>,

    // Coordinator 质量统计
    raw_finding_count: usize,
    after_dedup_count: usize,
    debate_triggered: usize,
    debate_changed: usize,
    blindspot_extra: usize,
    cross_agent_links: usize,
    legal_verify_count: usize,

    // Embedding 统计
    chunks_embedded: usize,
    embed_dimension: usize,
    embed_engine: String,
}

#[derive(Debug, Clone)]
struct RawStageRecord {
    stage: SemanticStage,
    duration_ms: u64,
    detail: StageDetail,
    sub_phases: Vec<CoordinatorPhaseRecord>,
}

impl MetricsCollector {
    pub fn new(schema_version: &str, model_name: &str) -> Self {
        Self {
            schema_version: schema_version.to_string(),
            model_name: model_name.to_string(),
            start_time: Instant::now(),
            stages: Vec::new(),
            llm_calls: Vec::new(),
            total_tokens_input: 0,
            total_tokens_output: 0,
            total_llm_duration_ms: 0,
            findings_detail: Vec::new(),
            agent_raw_findings: HashMap::new(),
            agent_high_findings: HashMap::new(),
            agent_medium_findings: HashMap::new(),
            agent_low_findings: HashMap::new(),
            agent_info_findings: HashMap::new(),
            agent_confidence_sum: HashMap::new(),
            agent_confidence_count: HashMap::new(),
            raw_finding_count: 0,
            after_dedup_count: 0,
            debate_triggered: 0,
            debate_changed: 0,
            blindspot_extra: 0,
            cross_agent_links: 0,
            legal_verify_count: 0,
            chunks_embedded: 0,
            embed_dimension: 0,
            embed_engine: String::new(),
        }
    }

    // ── Layer 1: 阶段计时 ──────────────────────────────────

    /// 记录一个语义阶段的耗时。
    pub fn record_stage(&mut self, stage: SemanticStage, duration_ms: u64, detail: StageDetail) {
        self.stages.push(RawStageRecord {
            stage,
            duration_ms,
            detail,
            sub_phases: Vec::new(),
        });
    }

    /// 为最近一个阶段追加子阶段记录（用于 Coordinator 内 7 阶段）。
    pub fn record_sub_phase(&mut self, phase_name: &str, duration_ms: u64) {
        if let Some(last) = self.stages.last_mut() {
            last.sub_phases.push(CoordinatorPhaseRecord {
                phase: phase_name.to_string(),
                duration_secs: duration_ms as f64 / 1000.0,
            });
        }
    }

    /// 更新最近一个阶段的耗时（用于 AgentReview 先占位后填充）。
    pub fn update_last_stage_duration(&mut self, duration_ms: u64) {
        if let Some(last) = self.stages.last_mut() {
            last.duration_ms = duration_ms;
        }
    }

    /// 获取从创建到现在的总耗时（ms）。
    pub fn elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    // ── Layer 2: LLM 调用 ──────────────────────────────────

    /// 记录单次 LLM 调用。
    pub fn record_llm_call(&mut self, call: LlmCallRecord) {
        self.total_tokens_input += call.tokens_input as u64;
        self.total_tokens_output += call.tokens_output as u64;
        self.total_llm_duration_ms += call.duration_ms;
        self.llm_calls.push(call);
    }

    /// 将最近一次 LLM 调用标记为 finding 解析成功。
    pub fn mark_last_finding_parsed_ok(&mut self) {
        if let Some(last) = self.llm_calls.last_mut() {
            last.finding_parsed_ok = true;
        }
    }

    // ── Layer 3: 审核质量 ──────────────────────────────────

    /// 记录单个 Agent 产出的 finding 统计。
    pub fn record_agent_findings(
        &mut self,
        agent_name: &str,
        findings: &[crate::agents::types::RiskFinding],
    ) {
        let raw = findings.len();
        let high = findings
            .iter()
            .filter(|f| f.severity == crate::agents::types::RiskSeverity::High)
            .count();
        let medium = findings
            .iter()
            .filter(|f| f.severity == crate::agents::types::RiskSeverity::Medium)
            .count();
        let low = findings
            .iter()
            .filter(|f| f.severity == crate::agents::types::RiskSeverity::Low)
            .count();
        let info = findings
            .iter()
            .filter(|f| f.severity == crate::agents::types::RiskSeverity::Info)
            .count();
        let confidence_sum: f64 = findings.iter().map(|f| f.confidence as f64).sum();
        let confidence_count = findings.len();

        *self
            .agent_raw_findings
            .entry(agent_name.to_string())
            .or_default() += raw;
        *self
            .agent_high_findings
            .entry(agent_name.to_string())
            .or_default() += high;
        *self
            .agent_medium_findings
            .entry(agent_name.to_string())
            .or_default() += medium;
        *self
            .agent_low_findings
            .entry(agent_name.to_string())
            .or_default() += low;
        *self
            .agent_info_findings
            .entry(agent_name.to_string())
            .or_default() += info;
        *self
            .agent_confidence_sum
            .entry(agent_name.to_string())
            .or_default() += confidence_sum;
        *self
            .agent_confidence_count
            .entry(agent_name.to_string())
            .or_default() += confidence_count;
    }

    /// 设置 Coordinator 级别的质量统计（在 review 完成后调用一次）。
    #[allow(clippy::too_many_arguments)]
    pub fn set_coordinator_stats(
        &mut self,
        raw_count: usize,
        after_dedup: usize,
        debate_triggered: usize,
        debate_changed: usize,
        blindspot_extra: usize,
        cross_agent_links: usize,
        legal_verify_count: usize,
    ) {
        self.raw_finding_count = raw_count;
        self.after_dedup_count = after_dedup;
        self.debate_triggered = debate_triggered;
        self.debate_changed = debate_changed;
        self.blindspot_extra = blindspot_extra;
        self.cross_agent_links = cross_agent_links;
        self.legal_verify_count = legal_verify_count;
    }

    // ── Layer 4: 资源 ──────────────────────────────────────

    /// 记录嵌入统计。
    pub fn set_embedding_stats(&mut self, chunk_count: usize, dimension: usize, engine: &str) {
        self.chunks_embedded = chunk_count;
        self.embed_dimension = dimension;
        self.embed_engine = engine.to_string();
    }

    /// 记录完整 RiskFinding 列表（用于 GUI 逐条审查）。
    pub fn set_findings_detail(&mut self, findings: &[crate::agents::types::RiskFinding]) {
        self.findings_detail = findings
            .iter()
            .map(|f| serde_json::to_value(f).unwrap_or_default())
            .collect();
    }

    // ── 最终聚合 ───────────────────────────────────────────

    /// 汇总所有采集数据，生成 RunMetrics。
    pub fn finalize(&self, meta: RunMeta) -> RunMetrics {
        let total_ms = self.elapsed_ms();

        // Layer 1
        let latency = self.build_latency_report(total_ms);

        // Layer 2
        let llm_efficiency = self.build_llm_efficiency_report();

        // Layer 3
        let review_quality = self.build_quality_report();

        // Layer 4
        let resources = self.build_resource_report();

        RunMetrics {
            schema_version: self.schema_version.clone(),
            meta,
            latency,
            llm_efficiency,
            review_quality,
            resources,
        }
    }

    fn build_latency_report(&self, total_ms: u64) -> LatencyReport {
        let total_secs = total_ms as f64 / 1000.0;
        let stages: Vec<StageRecord> = self
            .stages
            .iter()
            .map(|raw| {
                let duration_secs = raw.duration_ms as f64 / 1000.0;
                let pct_of_total = if total_secs > 0.0 {
                    (duration_secs / total_secs) * 100.0
                } else {
                    0.0
                };
                let mut detail = raw.detail.clone();

                // 如果有 coordinator 子阶段，注入到 AgentReview 详情中
                if !raw.sub_phases.is_empty()
                    && let StageDetail::AgentReview {
                        ref mut coordinator_phases,
                        ..
                    } = detail
                {
                    *coordinator_phases = Some(raw.sub_phases.clone());
                }

                StageRecord {
                    stage: raw.stage.clone(),
                    duration_secs,
                    pct_of_total: (pct_of_total * 10.0).round() / 10.0,
                    detail,
                }
            })
            .collect();

        LatencyReport {
            total_wall_clock_secs: (total_secs * 10.0).round() / 10.0,
            stages,
        }
    }

    fn build_llm_efficiency_report(&self) -> LlmEfficiencyReport {
        let total_calls = self.llm_calls.len();

        // 按 Agent 聚合
        let mut by_agent: HashMap<String, AgentLlmStats> = HashMap::new();
        let mut tool_totals = ToolUsageSummary::default();
        let mut total_wasted = 0usize;

        for call in &self.llm_calls {
            let entry = by_agent
                .entry(call.agent_name.clone())
                .or_insert_with(|| AgentLlmStats {
                    turns: 0,
                    llm_calls: 0,
                    tokens_input: 0,
                    tokens_output: 0,
                    tools: HashMap::new(),
                    findings_produced: 0,
                    wasted_calls: 0,
                    avg_duration_ms: 0.0,
                });

            entry.llm_calls += 1;
            entry.tokens_input += call.tokens_input as u64;
            entry.tokens_output += call.tokens_output as u64;

            // 统计工具调用
            for tool_name in &call.tools_called {
                *entry.tools.entry(tool_name.clone()).or_default() += 1;
                match tool_name.as_str() {
                    "search_document" => tool_totals.search_document += 1,
                    "read_section" => tool_totals.read_section += 1,
                    "search_knowledge" | "web_search" => tool_totals.search_knowledge += 1,
                    "output_finding" => tool_totals.output_finding += 1,
                    "answer_user" => tool_totals.answer_user += 1,
                    _ => tool_totals.other += 1,
                }
            }

            if call.produced_finding && call.finding_parsed_ok {
                entry.findings_produced += 1;
            }
            if !call.produced_finding {
                entry.wasted_calls += 1;
                total_wasted += 1;
            }
        }

        // 计算各 Agent 平均延迟 + 实际轮数
        for (name, stats) in &mut by_agent {
            let agent_calls: Vec<&LlmCallRecord> = self
                .llm_calls
                .iter()
                .filter(|c| c.agent_name == *name)
                .collect();
            if !agent_calls.is_empty() {
                let total_ms: u64 = agent_calls.iter().map(|c| c.duration_ms).sum();
                stats.avg_duration_ms = total_ms as f64 / agent_calls.len() as f64;
                // turns = 最大 turn 编号（代表执行了多少轮 ReAct 循环）
                stats.turns = agent_calls.iter().map(|c| c.turn).max().unwrap_or(0);
            }
        }

        let wasted_ratio = if total_calls > 0 {
            (total_wasted as f64 / total_calls as f64 * 100.0).round() / 100.0
        } else {
            0.0
        };

        let avg_api_ms = if total_calls > 0 {
            self.total_llm_duration_ms as f64 / total_calls as f64
        } else {
            0.0
        };

        // 成本估算
        let (input_price, output_price) = self.get_pricing();
        let cost_input = self.total_tokens_input as f64 / 1_000_000.0 * input_price;
        let cost_output = self.total_tokens_output as f64 / 1_000_000.0 * output_price;

        LlmEfficiencyReport {
            totals: LlmEfficiencyTotals {
                llm_calls: total_calls,
                tokens_input: self.total_tokens_input,
                tokens_output: self.total_tokens_output,
                cost_cny: ((cost_input + cost_output) * 100.0).round() / 100.0,
                avg_api_duration_ms: (avg_api_ms * 10.0).round() / 10.0,
            },
            by_agent,
            tool_usage: tool_totals,
            wasted_call_ratio: wasted_ratio,
            call_log: self.llm_calls.clone(),
        }
    }

    fn build_quality_report(&self) -> ReviewQualityReport {
        let total_raw = self.raw_finding_count;
        let after_dedup = self.after_dedup_count;
        let dedup_rate = if total_raw > 0 {
            ((total_raw - after_dedup) as f64 / total_raw as f64 * 100.0).round() / 100.0
        } else {
            0.0
        };

        // 按严重度汇总（使用 coordinator 级别的统计）
        let high = self.agent_high_findings.values().sum();
        let medium = self.agent_medium_findings.values().sum();
        let low = self.agent_low_findings.values().sum();
        let info = self.agent_info_findings.values().sum();

        // 平均置信度
        let total_conf: f64 = self.agent_confidence_sum.values().sum();
        let total_conf_count: usize = self.agent_confidence_count.values().sum();
        let avg_confidence = if total_conf_count > 0 {
            (total_conf / total_conf_count as f64 * 100.0).round() / 100.0
        } else {
            0.0
        };

        // 按 Agent 质量统计
        let mut by_agent: HashMap<String, AgentQualityStats> = HashMap::new();
        for name in self.agent_raw_findings.keys() {
            let raw = self.agent_raw_findings.get(name).copied().unwrap_or(0);
            let conf_sum = self.agent_confidence_sum.get(name).copied().unwrap_or(0.0);
            let conf_count = self.agent_confidence_count.get(name).copied().unwrap_or(0);
            let agent_avg_conf = if conf_count > 0 {
                (conf_sum / conf_count as f64 * 100.0).round() / 100.0
            } else {
                0.0
            };

            by_agent.insert(
                name.clone(),
                AgentQualityStats {
                    raw,
                    after_dedup: raw, // per-agent dedup 在 merge 阶段处理，此处用 raw 近似
                    high: self.agent_high_findings.get(name).copied().unwrap_or(0),
                    medium: self.agent_medium_findings.get(name).copied().unwrap_or(0),
                    low: self.agent_low_findings.get(name).copied().unwrap_or(0),
                    info: self.agent_info_findings.get(name).copied().unwrap_or(0),
                    avg_confidence: agent_avg_conf,
                },
            );
        }

        ReviewQualityReport {
            findings: FindingSummary {
                total_raw,
                after_dedup,
                dedup_rate,
                by_severity: SeverityBreakdown {
                    high,
                    medium,
                    low,
                    info,
                },
                avg_confidence,
                median_confidence: 0.0,
            },
            findings_detail: self.findings_detail.clone(),
            by_agent,
            coordinator: CoordinatorQualityStats {
                debate_triggered: self.debate_triggered,
                debate_changed_verdict: self.debate_changed,
                blindspot_extra_findings: self.blindspot_extra,
                cross_agent_links: self.cross_agent_links,
                legal_verify_count: self.legal_verify_count,
            },
        }
    }

    fn build_resource_report(&self) -> ResourceReport {
        let (input_price, output_price) = self.get_pricing();
        let cost_input = self.total_tokens_input as f64 / 1_000_000.0 * input_price;
        let cost_output = self.total_tokens_output as f64 / 1_000_000.0 * output_price;

        ResourceReport {
            tokens: TokenCostBreakdown {
                total_input: self.total_tokens_input,
                total_output: self.total_tokens_output,
                pricing_input_per_m: input_price,
                pricing_output_per_m: output_price,
                cost_input_cny: (cost_input * 100.0).round() / 100.0,
                cost_output_cny: (cost_output * 100.0).round() / 100.0,
                cost_total_cny: ((cost_input + cost_output) * 100.0).round() / 100.0,
            },
            memory: MemoryUsage {
                peak_mb: None,
                onnx_model_mb: None,
                doc_cache_mb: None,
            },
            embedding: EmbeddingStats {
                engine: self.embed_engine.clone(),
                chunks_embedded: self.chunks_embedded,
                duration_secs: 0.0, // 由阶段记录填充
                chunks_per_sec: 0.0,
                dimension: self.embed_dimension,
            },
        }
    }

    fn get_pricing(&self) -> (f64, f64) {
        if self.model_name.contains("turbo") {
            (QWEN_TURBO_INPUT_PRICE, QWEN_TURBO_OUTPUT_PRICE)
        } else {
            // qwen-plus 及其他默认定价
            (QWEN_PLUS_INPUT_PRICE, QWEN_PLUS_OUTPUT_PRICE)
        }
    }
}
