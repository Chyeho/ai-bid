package com.ithsd.smart_tender.model.vo;

import java.time.LocalDateTime;

/**
 * 追溯会话摘要 VO — 对应 GET /api/audit-tasks/{taskId}/traces 列表项。
 *
 * <p>设计文档 §10.1.5 — Session 摘要卡片。</p>
 */
public class TraceSessionVO {
    private String sessionId;
    private String agentName;
    private String clauseId;
    private String riskId;
    private String severity;          // high / medium / low / info
    private Double confidence;
    private String initialTier;
    private String finalTier;
    private Integer totalTurns;
    private Integer eventCount;
    private String status;            // running / completed / max_turns_exceeded / error
    private LocalDateTime startedAt;
    private LocalDateTime finishedAt;
    /** 从最后一个 output_finding 事件提取的摘要 */
    private String summary;

    public TraceSessionVO() {}

    public String getSessionId() { return sessionId; }
    public void setSessionId(String sessionId) { this.sessionId = sessionId; }

    public String getAgentName() { return agentName; }
    public void setAgentName(String agentName) { this.agentName = agentName; }

    public String getClauseId() { return clauseId; }
    public void setClauseId(String clauseId) { this.clauseId = clauseId; }

    public String getRiskId() { return riskId; }
    public void setRiskId(String riskId) { this.riskId = riskId; }

    public String getSeverity() { return severity; }
    public void setSeverity(String severity) { this.severity = severity; }

    public Double getConfidence() { return confidence; }
    public void setConfidence(Double confidence) { this.confidence = confidence; }

    public String getInitialTier() { return initialTier; }
    public void setInitialTier(String initialTier) { this.initialTier = initialTier; }

    public String getFinalTier() { return finalTier; }
    public void setFinalTier(String finalTier) { this.finalTier = finalTier; }

    public Integer getTotalTurns() { return totalTurns; }
    public void setTotalTurns(Integer totalTurns) { this.totalTurns = totalTurns; }

    public Integer getEventCount() { return eventCount; }
    public void setEventCount(Integer eventCount) { this.eventCount = eventCount; }

    public String getStatus() { return status; }
    public void setStatus(String status) { this.status = status; }

    public LocalDateTime getStartedAt() { return startedAt; }
    public void setStartedAt(LocalDateTime startedAt) { this.startedAt = startedAt; }

    public LocalDateTime getFinishedAt() { return finishedAt; }
    public void setFinishedAt(LocalDateTime finishedAt) { this.finishedAt = finishedAt; }

    public String getSummary() { return summary; }
    public void setSummary(String summary) { this.summary = summary; }
}
