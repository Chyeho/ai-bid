package com.ithsd.smart_tender.pojo.vo;

import java.util.List;

public class ResultVO {
    private String taskId;
    private String auditResult;
    private SummaryVO summary;
    private List<IssueVO> issues;

    public String getTaskId() {
        return taskId;
    }

    public void setTaskId(String taskId) {
        this.taskId = taskId;
    }

    public String getAuditResult() {
        return auditResult;
    }

    public void setAuditResult(String auditResult) {
        this.auditResult = auditResult;
    }

    public SummaryVO getSummary() {
        return summary;
    }

    public void setSummary(SummaryVO summary) {
        this.summary = summary;
    }

    public List<IssueVO> getIssues() {
        return issues;
    }

    public void setIssues(List<IssueVO> issues) {
        this.issues = issues;
    }
}
