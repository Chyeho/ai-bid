package com.ithsd.smart_tender.service.impl;

import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.ithsd.smart_tender.config.RustApiProperties;
import com.ithsd.smart_tender.mapper.AuditIssueMapper;
import com.ithsd.smart_tender.mapper.AuditTaskMapper;
import com.ithsd.smart_tender.model.entity.AuditIssue;
import com.ithsd.smart_tender.model.entity.AuditTask;
import com.ithsd.smart_tender.model.enums.AuditStageEnum;
import com.ithsd.smart_tender.model.enums.AuditTaskStatusEnum;
import com.ithsd.smart_tender.model.enums.SseEventTypeEnum;
import com.ithsd.smart_tender.model.vo.AuditCompleteVO;
import com.ithsd.smart_tender.model.vo.AuditTaskStatusVO;
import com.ithsd.smart_tender.model.vo.IssueVO;
import com.ithsd.smart_tender.model.vo.SummaryVO;
import com.ithsd.smart_tender.service.AuditEngineService;
import com.ithsd.smart_tender.service.TraceService;
import com.ithsd.smart_tender.service.engine.rust.RustApiClient;
import com.ithsd.smart_tender.service.engine.rust.RustDocumentService;
import com.ithsd.smart_tender.service.engine.rust.RustSseClient;
import com.ithsd.smart_tender.model.dto.rust.RustReviewAcceptedResponse;
import com.ithsd.smart_tender.model.dto.rust.RustReviewRequest;
import com.ithsd.smart_tender.model.dto.rust.RustReviewResponse;
import com.ithsd.smart_tender.model.dto.rust.RustReviewResultResponse;
import com.ithsd.smart_tender.model.dto.rust.RustRiskFinding;
import com.ithsd.smart_tender.sse.AuditTaskEventService;
import com.ithsd.smart_tender.sse.SseHub;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.scheduling.annotation.Async;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Supplier;
import java.util.stream.Collectors;

/**
 * 审核引擎 — 委托 Rust Multi-Agent 引擎执行审核。
 * <p>整个管线（extract → chunk → embed → review）在 Rust 侧同步完成，
 * Java 仅负责：上传文件 → 调用审核 → 映射结果 → 发射 SSE。</p>
 */
@Service
public class AuditEngineServiceImpl implements AuditEngineService {

    private static final Set<String> RUNNING_TASKS = ConcurrentHashMap.newKeySet();
    private static final Logger log = LoggerFactory.getLogger(AuditEngineServiceImpl.class);

    private final AuditTaskMapper auditTaskMapper;
    private final AuditIssueMapper auditIssueMapper;
    private final SseHub sseHub;
    private final AuditTaskEventService eventService;
    private final RustApiClient rustApiClient;
    private final RustDocumentService rustDocumentService;
    private final RustSseClient rustSseClient;
    private final RustApiProperties rustApiProperties;
    private final TraceService traceService;
    private final ObjectMapper objectMapper = new ObjectMapper()
            .setPropertyNamingStrategy(
                com.fasterxml.jackson.databind.PropertyNamingStrategies.SNAKE_CASE);

    public AuditEngineServiceImpl(
            AuditTaskMapper auditTaskMapper,
            AuditIssueMapper auditIssueMapper,
            SseHub sseHub,
            AuditTaskEventService eventService,
            RustApiClient rustApiClient,
            RustDocumentService rustDocumentService,
            RustSseClient rustSseClient,
            RustApiProperties rustApiProperties,
            TraceService traceService
    ) {
        this.auditTaskMapper = auditTaskMapper;
        this.auditIssueMapper = auditIssueMapper;
        this.sseHub = sseHub;
        this.eventService = eventService;
        this.rustApiClient = rustApiClient;
        this.rustDocumentService = rustDocumentService;
        this.rustSseClient = rustSseClient;
        this.rustApiProperties = rustApiProperties;
        this.traceService = traceService;
    }

    @Override
    @Async("auditTaskExecutor")
    @Transactional
    public void start(String taskId) {
        log.info("audit async task started: taskId={}, thread={}", taskId, Thread.currentThread().getName());
        if (!RUNNING_TASKS.add(taskId)) {
            log.warn("audit task skipped due to concurrent start, taskId={}", taskId);
            return;
        }
        try {
            runEngine(taskId);
        } finally {
            RUNNING_TASKS.remove(taskId);
        }
    }

    // ── 主流程 ──────────────────────────────────────────────────────

    private AuditTask loadTask(String taskId) {
        return auditTaskMapper.selectOne(
                new LambdaQueryWrapper<AuditTask>().eq(AuditTask::getTaskId, taskId));
    }

    private void runEngine(String taskId) {
        AuditTask task = loadTask(taskId);
        if (task == null) {
            log.warn("audit task not found, taskId={}", taskId);
            sseHub.close(taskId);
            return;
        }
        if (!AuditTaskStatusEnum.PENDING.getCode().equals(task.getTaskStatus())) {
            log.info("audit task already started or finished, taskId={}, status={}", taskId, task.getTaskStatus());
            sseHub.close(taskId);
            return;
        }

        try {
            // Stage 1: 上传文件到 Rust（幂等）
            log.info("═══ [审核 Stage 1/4] 开始上传文件到 Rust 引擎: taskId={}, bidId={} ═══", taskId, task.getBidId());
            updateStage(task, AuditStageEnum.UPLOADING, 10);
            String rustDocId;
            try {
                rustDocId = rustDocumentService.ensureUploaded(task.getBidId());
                log.info("═══ [审核 Stage 1/4] 文件上传完成 → rustDocId={} ═══", rustDocId);
            } catch (Exception ex) {
                log.error("❌ [审核 Stage 1/4] Rust 上传失败: taskId={}, bidId={} — {}", taskId, task.getBidId(), ex.getMessage(), ex);
                failTask(task, "文件上传 Rust 失败: " + ex.getMessage());
                return;
            }

            // Stage 2: 异步调用 Rust Multi-Agent 审核（SSE 实时推送 + 异步结果获取）
            log.info("═══ [审核 Stage 2/4] 开始异步调用 Rust Multi-Agent 审核引擎: rustDocId={} ═══", rustDocId);
            updateStage(task, AuditStageEnum.REVIEWING, 30);
            RustReviewRequest reviewReq = new RustReviewRequest();
            reviewReq.setMaxClauses(200);
            if (task.getEnabledChecks() != null && !task.getEnabledChecks().isEmpty()) {
                reviewReq.setEnabledAgents(task.getEnabledChecks());
            }

            // 信号量：SSE 回调 → 主流程 await
            CompletableFuture<Void> reviewDoneSignal = new CompletableFuture<>();
            CompletableFuture<String> reviewErrorSignal = new CompletableFuture<>();

            // 启动 Rust SSE 实时推送 relay（在调用 Rust POST /review 之前连接，
            // 确保不丢失早期事件）
            CompletableFuture<Void> sseRelay = rustSseClient.connect(rustDocId, (eventType, data) -> {
                try {
                    switch (eventType) {
                        case "agent_progress" -> {
                            com.ithsd.smart_tender.model.vo.AgentProgressVO vo =
                                objectMapper.convertValue(data, com.ithsd.smart_tender.model.vo.AgentProgressVO.class);
                            emitSafe(taskId, SseEventTypeEnum.AGENT_PROGRESS, vo);
                        }
                        case "trace" -> {
                            com.ithsd.smart_tender.model.vo.TraceEventVO vo =
                                objectMapper.convertValue(data, com.ithsd.smart_tender.model.vo.TraceEventVO.class);
                            emitSafe(taskId, SseEventTypeEnum.TRACE, vo);
                            // 持久化到 trace_sessions / trace_events
                            try {
                                traceService.ingestTraceEvent(taskId, rustDocId, vo);
                            } catch (Exception e) {
                                log.warn("Trace ingest failed: clause={} turn={}", vo.getClauseId(), vo.getTurn(), e);
                            }
                        }
                        case "phase" -> {
                            com.ithsd.smart_tender.model.vo.PhaseVO vo =
                                objectMapper.convertValue(data, com.ithsd.smart_tender.model.vo.PhaseVO.class);
                            emitSafe(taskId, SseEventTypeEnum.PHASE, vo);
                            String phase = data.has("phase") ? data.get("phase").asText() : "";
                            updateStage(task, AuditStageEnum.REVIEWING,
                                "execute".equals(phase) ? 35 : "merge".equals(phase) ? 45 : "legal_verify".equals(phase) ? 55 : 60);
                        }
                        case "stats" -> {
                            emitSafe(taskId, SseEventTypeEnum.STATS, data);
                        }
                        case "finding_added" -> {
                            try {
                                RustRiskFinding rf = objectMapper.convertValue(data, RustRiskFinding.class);
                                if (!rf.shouldSkip()) {
                                    emitSafe(taskId, SseEventTypeEnum.ISSUE, toIssueVO(rf));
                                }
                            } catch (Exception ignored) {
                                log.debug("SSE finding_added map failed: {}", ignored.getMessage());
                            }
                        }
                        case "done" -> {
                            log.info("Rust SSE done received: docId={}", rustDocId);
                            // 标记所有 running session 为 completed
                            try {
                                traceService.markSessionsCompleted(taskId);
                            } catch (Exception e) {
                                log.warn("Trace markSessionsCompleted failed: taskId={}", taskId, e);
                            }
                            reviewDoneSignal.complete(null);
                        }
                        case "error" -> {
                            String msg = data.has("message") ? data.get("message").asText() : "审核引擎未知错误";
                            log.error("Rust SSE error received: docId={}, msg={}", rustDocId, msg);
                            reviewErrorSignal.complete(msg);
                        }
                        default -> {
                            log.debug("Rust SSE unknown event: {}", eventType);
                        }
                    }
                } catch (Exception e) {
                    log.debug("SSE relay event process failed: {}", e.getMessage());
                }
            });

            // 等待 SSE 连接就绪（避免丢失早期事件）
            waitForSseConnection(sseRelay);

            // 启动异步审核（202 Accepted）
            RustReviewAcceptedResponse accepted;
            try {
                accepted = rustApiClient.startReview(rustDocId, reviewReq);
            } catch (Exception ex) {
                log.error("❌ [审核 Stage 2/4] Rust 审核启动失败: taskId={}, rustDocId={} — {}", taskId, rustDocId, ex.getMessage(), ex);
                failTask(task, "Rust 审核启动失败: " + ex.getMessage());
                return;
            }
            if (accepted.isConflict()) {
                log.warn("Rust review conflict, retrying with wait: docId={}", rustDocId);
                try { Thread.sleep(3000); } catch (InterruptedException ignored) {}
                try {
                    accepted = rustApiClient.startReview(rustDocId, reviewReq);
                } catch (Exception ex) {
                    log.error("❌ [审核 Stage 2/4] Rust 审核重试失败: taskId={}, rustDocId={} — {}", taskId, rustDocId, ex.getMessage(), ex);
                    failTask(task, "Rust 审核启动失败（重试）: " + ex.getMessage());
                    return;
                }
                if (accepted.isConflict()) {
                    log.error("❌ [审核 Stage 2/4] Rust 审核仍冲突: taskId={}, rustDocId={}", taskId, rustDocId);
                    failTask(task, "该文档已有进行中的审核任务，请稍后重试");
                    return;
                }
            }
            log.info("═══ [审核 Stage 2/4] Rust 异步审核已提交，等待完成... ═══");

            // 等待审核完成（SSE "done" 信号 或 "error" 信号）
            RustReviewResponse reviewResp = awaitReviewResult(rustDocId, reviewDoneSignal, reviewErrorSignal);
            if (reviewResp == null) {
                log.error("❌ [审核 Stage 2/4] Rust 审核结果获取失败: taskId={}, rustDocId={}", taskId, rustDocId);
                return;
            }
            log.info("═══ [审核 Stage 2/4] Rust 审核引擎返回: findings={} ═══",
                reviewResp.getFindings() != null ? reviewResp.getFindings().size() : 0);

            // Stage 3: findings → SSE 实时推送到前端 + 持久化到 DB
            log.info("═══ [审核 Stage 3/4] 实时推送 findings → SSE + 入库 ═══");
            updateStage(task, AuditStageEnum.REVIEWING, 70);
            List<RustRiskFinding> activeFindings = new ArrayList<>();
            if (reviewResp.getFindings() != null) {
                // 先删除旧数据（幂等）
                auditIssueMapper.delete(new LambdaQueryWrapper<AuditIssue>()
                        .eq(AuditIssue::getAuditId, task.getId()));
                int seq = 0;
                for (RustRiskFinding finding : reviewResp.getFindings()) {
                    if (finding.shouldSkip()) continue;
                    activeFindings.add(finding);
                    emitSafe(task.getTaskId(), SseEventTypeEnum.ISSUE, toIssueVO(finding));
                    // 持久化到 audit_issue 表
                    try {
                        AuditIssue issue = AuditIssue.builder()
                                .auditId(task.getId())
                                .issueNo("ISSUE-" + (finding.getRiskId() != null ? finding.getRiskId() : String.valueOf(++seq)))
                                .severity(finding.mappedSeverity())
                                .category(finding.getRiskType())
                                .description(finding.getReason() != null ? finding.getReason() : finding.getRiskType())
                                .suggestion(finding.getSuggestion())
                                .pageNumber(finding.getPageNumber())
                                .sectionName(finding.getSectionPath() != null
                                        ? String.join(" > ", finding.getSectionPath()) : null)
                                .context(finding.getContext() != null ? finding.getContext() : finding.getSourceQuote())
                                .reference(finding.getLegalBasis() != null && !finding.getLegalBasis().isEmpty()
                                        ? String.join("; ", finding.getLegalBasis()) : null)
                                .createTime(LocalDateTime.now())
                                .build();
                        auditIssueMapper.insert(issue);
                    } catch (Exception e) {
                        log.warn("Failed to persist finding {}: {}", finding.getRiskId(), e.getMessage());
                    }
                }
            }

            // Stage 4: 完成
            task.setTaskStatus(AuditTaskStatusEnum.COMPLETED.getCode());
            task.setStage(AuditStageEnum.SUMMARY.name());
            task.setProgress(100);
            task.setEndTime(LocalDateTime.now());
            task.setUpdatedAt(LocalDateTime.now());
            auditTaskMapper.updateById(task);
            emitSafe(taskId, SseEventTypeEnum.COMPLETE, toCompleteVO(task, activeFindings, reviewResp));
            log.info("═══ [审核 Stage 4/4] ✅ 审核完成: taskId={}, findings={} (high={}, medium={}, low={}, info={}) ═══",
                    taskId, activeFindings.size(),
                    activeFindings.stream().filter(i -> "high".equals(i.mappedSeverity())).count(),
                    activeFindings.stream().filter(i -> "medium".equals(i.mappedSeverity())).count(),
                    activeFindings.stream().filter(i -> "low".equals(i.mappedSeverity())).count(),
                    activeFindings.stream().filter(i -> "info".equals(i.mappedSeverity())).count());

        } catch (Exception ex) {
            log.error("❌ [审核] 未预期的异常导致审核失败: taskId={} — {}", taskId, ex.getMessage(), ex);
            failTask(task, crop(ex.getMessage()));
        } finally {
            sseHub.close(taskId);
        }
    }

    // ── 映射：RustRiskFinding → IssueVO（SSE 推送用） ────────────────

    private IssueVO toIssueVO(RustRiskFinding f) {
        IssueVO vo = new IssueVO();
        vo.setIssueNo("ISSUE-" + (f.getRiskId() != null ? f.getRiskId() : "?"));
        vo.setRiskId(f.getRiskId());
        vo.setSeverity(f.mappedSeverity());
        vo.setCategory(f.getRiskType());
        vo.setAgentName(f.getAgent());
        vo.setDescription(f.getReason() != null ? f.getReason() : f.getRiskType());
        vo.setSuggestion(f.getSuggestion());
        vo.setSourceQuote(f.getSourceQuote());
        vo.setLegalBasis(f.getLegalBasis());
        vo.setCaseRefs(f.getCaseRefs());
        vo.setConfidence(f.getConfidence() > 0 ? f.getConfidence() : null);

        IssueVO.LocationVO loc = new IssueVO.LocationVO();
        if (f.getPageNumber() != null) loc.setPageNumber(f.getPageNumber());
        if (f.getSectionPath() != null && !f.getSectionPath().isEmpty())
            loc.setSectionName(String.join(" > ", f.getSectionPath()));
        loc.setContext(f.getContext() != null ? f.getContext() : f.getSourceQuote());
        vo.setLocation(loc);

        if (f.getLegalBasis() != null && !f.getLegalBasis().isEmpty())
            vo.setReference(String.join("; ", f.getLegalBasis()));

        if (f.getPageNumber() != null) {
            vo.setAnchorPage(f.getPageNumber());
        }
        if (f.getSectionPath() != null && !f.getSectionPath().isEmpty()) {
            vo.setAnchorSection(String.join(" > ", f.getSectionPath()));
        }

        vo.setNoRisk(f.isNoRisk());
        vo.setInitialTier(f.getInitialTier());
        vo.setFinalTier(f.getFinalTier());
        vo.setTierEscalated(f.isTierEscalated());
        vo.setTruncated(f.isTruncated());
        vo.setClauseIds(f.getClauseIds());
        vo.setBlockIds(f.getBlockIds());
        vo.setAgent(f.getAgent());

        if (f.getCitations() != null && !f.getCitations().isEmpty()) {
            List<IssueVO.CitationVO> citations = f.getCitations().stream().map(c -> {
                IssueVO.CitationVO cv = new IssueVO.CitationVO();
                cv.setTitle(c.getTitle());
                cv.setUrl(c.getUrl());
                cv.setSiteName(c.getSiteName());
                return cv;
            }).collect(Collectors.toList());
            vo.setCitations(citations);
        }

        if (f.getSuggestedAgent() != null) {
            IssueVO.SuggestedAgentVO sa = new IssueVO.SuggestedAgentVO();
            sa.setAgentName(f.getSuggestedAgent().getAgentName());
            sa.setAgentPrompt(f.getSuggestedAgent().getAgentPrompt());
            sa.setSectionKeywords(f.getSuggestedAgent().getSectionKeywords());
            sa.setReason(f.getSuggestedAgent().getReason());
            vo.setSuggestedAgent(sa);
        }

        return vo;
    }

    // ── 状态更新 & SSE ──────────────────────────────────────────────

    private void updateStage(AuditTask task, AuditStageEnum stage, int progress) {
        task.setTaskStatus(AuditTaskStatusEnum.PROCESSING.getCode());
        task.setStage(stage.name());
        task.setProgress(progress);
        if (task.getStartTime() == null) task.setStartTime(LocalDateTime.now());
        task.setUpdatedAt(LocalDateTime.now());
        auditTaskMapper.updateById(task);
        emitSafe(task.getTaskId(), SseEventTypeEnum.PROGRESS, toStatusVO(task));
    }

    private void failTask(AuditTask task, String errorMsg) {
        task.setTaskStatus(AuditTaskStatusEnum.FAILED.getCode());
        task.setErrorMsg(crop(errorMsg));
        task.setEndTime(LocalDateTime.now());
        task.setUpdatedAt(LocalDateTime.now());
        auditTaskMapper.updateById(task);
        emitSafe(task.getTaskId(), SseEventTypeEnum.COMPLETE, toCompleteVO(task, List.of(), null));
    }

    private void emitSafe(String taskId, SseEventTypeEnum eventType, Object payload) {
        try {
            String eventId = eventService.persist(taskId, eventType, payload);
            sseHub.emit(taskId, eventType, payload, eventId);
        } catch (Exception ex) {
            log.warn("emit sse failed: taskId={}, event={}", taskId, eventType.getEventName(), ex);
        }
    }

    private String crop(String value) {
        if (!StringUtils.hasText(value)) return "引擎执行失败";
        return value.length() <= 1000 ? value : value.substring(0, 1000);
    }

    // ── 异步审核等待 ──────────────────────────────────────────────────

    private void waitForSseConnection(CompletableFuture<Void> sseRelay) {
        try {
            sseRelay.get(15, java.util.concurrent.TimeUnit.SECONDS);
        } catch (Exception e) {
            log.warn("Rust SSE connect timeout, continuing without real-time events: {}", e.getMessage());
        }
    }

    private RustReviewResponse awaitReviewResult(
            String rustDocId,
            CompletableFuture<Void> reviewDoneSignal,
            CompletableFuture<String> reviewErrorSignal) {

        Supplier<RustReviewResultResponse> fetchResult = () -> {
            try {
                return rustApiClient.getReviewResult(rustDocId);
            } catch (Exception e) {
                log.warn("Rust getReviewResult failed: docId={}, {}", rustDocId, e.getMessage());
                return null;
            }
        };

        try {
            CompletableFuture.anyOf(reviewDoneSignal, reviewErrorSignal)
                .get(rustApiProperties.getReviewTimeoutMinutes(), java.util.concurrent.TimeUnit.MINUTES);

            if (reviewErrorSignal.isDone()) {
                String errMsg;
                try {
                    errMsg = reviewErrorSignal.get();
                } catch (Exception e) {
                    errMsg = "审核引擎错误（无法获取详情）";
                }
                log.error("Rust review engine error via SSE: docId={}, {}", rustDocId, errMsg);
                RustReviewResultResponse r = fetchResult.get();
                if (r != null && r.isFailed()) {
                    log.error("Rust review failed confirmed: docId={}, error={}", rustDocId, r.getError());
                }
                return null;
            }

            log.info("Rust review SSE done triggered, fetching result: docId={}", rustDocId);
            RustReviewResultResponse r = fetchResult.get();
            if (r != null && r.isCompleted()) {
                return r.getResult();
            }
            try { Thread.sleep(500); } catch (InterruptedException ignored) {}
            r = fetchResult.get();
            if (r != null && r.isCompleted()) {
                return r.getResult();
            }
            log.error("Rust review result not ready after done event: docId={}", rustDocId);
            return null;

        } catch (java.util.concurrent.TimeoutException e) {
            log.warn("Rust review SSE signal timeout after {}min, falling back to polling: docId={}",
                rustApiProperties.getReviewTimeoutMinutes(), rustDocId);

            int maxPolls = 3;
            long pollIntervalMs = 5000;
            for (int i = 0; i < maxPolls; i++) {
                RustReviewResultResponse r = fetchResult.get();
                if (r != null && r.isCompleted()) {
                    log.info("Rust review result obtained via polling (attempt {}): docId={}", i + 1, rustDocId);
                    return r.getResult();
                }
                if (r != null && r.isFailed()) {
                    log.error("Rust review failed (polling): docId={}, error={}", rustDocId, r.getError());
                    return null;
                }
                if (i < maxPolls - 1) {
                    try { Thread.sleep(pollIntervalMs); } catch (InterruptedException ignored) {}
                }
            }
            log.error("Rust review result not ready after {} polling attempts: docId={}", maxPolls, rustDocId);
            return null;
        } catch (Exception e) {
            log.error("Rust review await interrupted: docId={}, {}", rustDocId, e.getMessage());
            return null;
        }
    }

    // ── VO 构建 ─────────────────────────────────────────────────────

    private AuditTaskStatusVO toStatusVO(AuditTask task) {
        AuditTaskStatusVO vo = new AuditTaskStatusVO();
        vo.setTaskId(task.getTaskId());
        vo.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        vo.setStage(task.getStage());
        vo.setProgress(task.getProgress());
        vo.setIssueCount(0);
        vo.setFailedStages(task.getFailedStages() == null ? List.of() : task.getFailedStages());
        return vo;
    }

    private AuditCompleteVO toCompleteVO(AuditTask task, List<RustRiskFinding> findings,
                                          RustReviewResponse reviewResp) {
        AuditCompleteVO vo = new AuditCompleteVO();
        vo.setTaskId(task.getTaskId());
        vo.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        vo.setAuditResult(findings.isEmpty() ? "pass" : "revise");
        vo.setIssueCount(findings.size());
        vo.setFailedStages(task.getFailedStages() == null ? List.of() : task.getFailedStages());
        vo.setSummary(buildSummary(findings));
        if (reviewResp != null && reviewResp.getRoutingSummary() != null) {
            vo.setRoutingSummary(buildRoutingSummary(reviewResp));
        }
        if (reviewResp != null && reviewResp.getGraphSnapshot() != null) {
            vo.setGraphSnapshot(reviewResp.getGraphSnapshot());
        }
        return vo;
    }

    private SummaryVO buildRoutingSummary(RustReviewResponse reviewResp) {
        SummaryVO s = new SummaryVO();
        if (reviewResp.getRoutingSummary() == null) return s;
        s.setTotalClauses(reviewResp.getRoutingSummary().getTotalClauses());
        s.setAgentClauseCounts(reviewResp.getRoutingSummary().getAgentClauseCounts());
        s.setLegalVerifyCount(reviewResp.getRoutingSummary().getLegalVerifyCount());
        s.setBlindSpotFindings(reviewResp.getRoutingSummary().getBlindSpotFindings());
        return s;
    }

    private SummaryVO buildSummary(List<RustRiskFinding> findings) {
        SummaryVO s = new SummaryVO();
        int high = 0, medium = 0, low = 0, info = 0;
        for (RustRiskFinding f : findings) {
            String sev = f.mappedSeverity();
            switch (sev) {
                case "high" -> high++;
                case "medium" -> medium++;
                case "low" -> low++;
                default -> info++;
            }
        }
        s.setTotalIssues(findings.size());
        s.setHigh(high);
        s.setMedium(medium);
        s.setLow(low);
        s.setInfo(info);
        return s;
    }
}
