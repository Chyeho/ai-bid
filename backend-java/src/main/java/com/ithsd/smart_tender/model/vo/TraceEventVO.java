package com.ithsd.smart_tender.model.vo;

import com.fasterxml.jackson.databind.JsonNode;

/**
 * ReAct 实时动态 VO — 对应 Rust ReviewEvent::Trace。
 * 前端用此渲染实时审查动态流（思考/工具调用/结果）。
 *
 * <p>设计文档 §10.1.1 TraceEvent。</p>
 */
public class TraceEventVO {
    // ── 基础字段（Rust SSE 提供）──
    private String eventType;   // turn_start / agent_thought / tool_call / tool_result / output_finding / agent_bus_send / agent_bus_recv
    private String agentName;
    private int turn;
    private String clauseId;
    private String summary;     // ≤200 字摘要
    private JsonNode payload;   // 可选结构化数据
    private String timestamp;

    // ── 扩展字段（Java 层补充，用于持久化）──
    private String eventId;     // UUID，Java 层生成
    private String sessionId;   // 所属 session UUID，Java 层推导
    private String riskId;      // 从 output_finding payload 提取

    public TraceEventVO() {}

    // ── Getters / Setters ──

    public String getEventType() { return eventType; }
    public void setEventType(String eventType) { this.eventType = eventType; }

    public String getAgentName() { return agentName; }
    public void setAgentName(String agentName) { this.agentName = agentName; }

    public int getTurn() { return turn; }
    public void setTurn(int turn) { this.turn = turn; }

    public String getClauseId() { return clauseId; }
    public void setClauseId(String clauseId) { this.clauseId = clauseId; }

    public String getSummary() { return summary; }
    public void setSummary(String summary) { this.summary = summary; }

    public JsonNode getPayload() { return payload; }
    public void setPayload(JsonNode payload) { this.payload = payload; }

    public String getTimestamp() { return timestamp; }
    public void setTimestamp(String timestamp) { this.timestamp = timestamp; }

    public String getEventId() { return eventId; }
    public void setEventId(String eventId) { this.eventId = eventId; }

    public String getSessionId() { return sessionId; }
    public void setSessionId(String sessionId) { this.sessionId = sessionId; }

    public String getRiskId() { return riskId; }
    public void setRiskId(String riskId) { this.riskId = riskId; }
}
