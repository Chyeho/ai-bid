package com.ithsd.smart_tender.model.vo;

import java.util.List;
import java.util.Map;

/**
 * 审核结果 VO — 对齐 Rust CoordinatorOutput。
 */
public class ResultVO {
    private String taskId;
    private String auditResult;
    private SummaryVO summary;
    private List<IssueVO> issues;

    // ── Rust CoordinatorOutput 扩展字段 ──
    /** 路由统计摘要（含 agentClauseCounts / legalVerifyCount / blindSpotFindings） */
    private SummaryVO routingSummary;
    /** 会话知识图谱快照（Map 形式，前端 GraphSnapshot 类型） */
    private Map<String, Object> graphSnapshot;

    public String getTaskId() { return taskId; }
    public void setTaskId(String taskId) { this.taskId = taskId; }
    public String getAuditResult() { return auditResult; }
    public void setAuditResult(String auditResult) { this.auditResult = auditResult; }
    public SummaryVO getSummary() { return summary; }
    public void setSummary(SummaryVO summary) { this.summary = summary; }
    public List<IssueVO> getIssues() { return issues; }
    public void setIssues(List<IssueVO> issues) { this.issues = issues; }
    public SummaryVO getRoutingSummary() { return routingSummary; }
    public void setRoutingSummary(SummaryVO routingSummary) { this.routingSummary = routingSummary; }
    public Map<String, Object> getGraphSnapshot() { return graphSnapshot; }
    public void setGraphSnapshot(Map<String, Object> graphSnapshot) { this.graphSnapshot = graphSnapshot; }
}
