package com.ithsd.smart_tender.model.vo;

public class AuditTaskCreateVO {
    private String taskId;

    public AuditTaskCreateVO() {
    }

    public AuditTaskCreateVO(String taskId) {
        this.taskId = taskId;
    }

    public String getTaskId() {
        return taskId;
    }

    public void setTaskId(String taskId) {
        this.taskId = taskId;
    }
}
