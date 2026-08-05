package com.ithsd.smart_tender.model.vo;

import java.util.List;

public class AuditTaskStatusVO {
    private String taskId;
    private String status;
    private String stage;
    private Integer progress;
    private Integer issueCount;
    private List<String> failedStages;
    private Long totalFileCount;
    private Long pendingFileCount;
    private Long processingFileCount;
    private Long failedFileCount;

    public String getTaskId() {
        return taskId;
    }

    public void setTaskId(String taskId) {
        this.taskId = taskId;
    }

    public String getStatus() {
        return status;
    }

    public void setStatus(String status) {
        this.status = status;
    }

    public String getStage() {
        return stage;
    }

    public void setStage(String stage) {
        this.stage = stage;
    }

    public Integer getProgress() {
        return progress;
    }

    public void setProgress(Integer progress) {
        this.progress = progress;
    }

    public Integer getIssueCount() {
        return issueCount;
    }

    public void setIssueCount(Integer issueCount) {
        this.issueCount = issueCount;
    }

    public List<String> getFailedStages() {
        return failedStages;
    }

    public void setFailedStages(List<String> failedStages) {
        this.failedStages = failedStages;
    }

    public Long getTotalFileCount() {
        return totalFileCount;
    }

    public void setTotalFileCount(Long totalFileCount) {
        this.totalFileCount = totalFileCount;
    }

    public Long getPendingFileCount() {
        return pendingFileCount;
    }

    public void setPendingFileCount(Long pendingFileCount) {
        this.pendingFileCount = pendingFileCount;
    }

    public Long getProcessingFileCount() {
        return processingFileCount;
    }

    public void setProcessingFileCount(Long processingFileCount) {
        this.processingFileCount = processingFileCount;
    }

    public Long getFailedFileCount() {
        return failedFileCount;
    }

    public void setFailedFileCount(Long failedFileCount) {
        this.failedFileCount = failedFileCount;
    }
}
