package com.ithsd.smart_tender.model.vo;

import java.util.List;
import java.util.Map;

public class AuditCompleteVO {
    private String taskId;
    private String status;
    private String auditResult;
    private Integer issueCount;
    private List<String> failedStages;
    private SummaryVO summary;
    private SummaryVO routingSummary;
    private Map<String, Object> graphSnapshot;

    public String getTaskId() { return taskId; }
    public void setTaskId(String taskId) { this.taskId = taskId; }
    public String getStatus() { return status; }
    public void setStatus(String status) { this.status = status; }
    public String getAuditResult() { return auditResult; }
    public void setAuditResult(String auditResult) { this.auditResult = auditResult; }
    public Integer getIssueCount() { return issueCount; }
    public void setIssueCount(Integer issueCount) { this.issueCount = issueCount; }
    public List<String> getFailedStages() { return failedStages; }
    public void setFailedStages(List<String> failedStages) { this.failedStages = failedStages; }
    public SummaryVO getSummary() { return summary; }
    public void setSummary(SummaryVO summary) { this.summary = summary; }
    public SummaryVO getRoutingSummary() { return routingSummary; }
    public void setRoutingSummary(SummaryVO routingSummary) { this.routingSummary = routingSummary; }
    public Map<String, Object> getGraphSnapshot() { return graphSnapshot; }
    public void setGraphSnapshot(Map<String, Object> graphSnapshot) { this.graphSnapshot = graphSnapshot; }
}
