package com.ithsd.smart_tender.pojo.entity;

import com.ithsd.smart_tender.repository.converter.StringListJsonConverter;
import jakarta.persistence.*;
import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;

@Entity
@Table(name = "audit_task", indexes = {
        @Index(name = "idx_audittask_task_id", columnList = "task_id", unique = true),
        @Index(name = "idx_audittask_bid_id", columnList = "bid_id"),
        @Index(name = "idx_audittask_task_status", columnList = "task_status"),
        @Index(name = "idx_audittask_audit_user_id", columnList = "audit_user_id"),
        @Index(name = "idx_audittask_create_time", columnList = "create_time")
})
public class AuditTaskEntity {
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(name = "task_id", nullable = false, unique = true, length = 64)
    private String taskId;

    @Column(name = "bid_id")
    private Long bidId;

    @Column(name = "task_status", nullable = false, columnDefinition = "TINYINT")
    private Integer taskStatus;

    @Column(name = "audit_result", length = 20)
    private String auditResult;

    @Column(name = "issue_count", nullable = false)
    private Integer issueCount;

    @Column(name = "critical_count", nullable = false)
    private Integer criticalCount;

    @Column(name = "warning_count", nullable = false)
    private Integer warningCount;

    @Column(name = "info_count", nullable = false)
    private Integer infoCount;

    @Column(name = "start_time")
    private LocalDateTime startTime;

    @Column(name = "end_time")
    private LocalDateTime endTime;

    @Column(name = "audit_user_id")
    private Long auditUserId;

    @Column(name = "create_time", nullable = false)
    private LocalDateTime createTime;

    // Extra fields to support application logic
    @Column(name = "stage", length = 64)
    private String stage;

    @Column(name = "progress", nullable = false, columnDefinition = "TINYINT")
    private Integer progress;

    @Convert(converter = StringListJsonConverter.class)
    @Column(name = "enabled_checks", columnDefinition = "json")
    private List<String> enabledChecks;

    @Convert(converter = StringListJsonConverter.class)
    @Column(name = "failed_stages", columnDefinition = "json")
    private List<String> failedStages;

    @Column(name = "error_msg", length = 1000)
    private String errorMsg;

    @Column(name = "updated_at")
    private LocalDateTime updatedAt;

    @Version
    @Column(name = "version", nullable = false)
    private Long version;

    public AuditTaskEntity() {
        this.enabledChecks = new ArrayList<>();
        this.failedStages = new ArrayList<>();
        this.issueCount = 0;
        this.criticalCount = 0;
        this.warningCount = 0;
        this.infoCount = 0;
        this.progress = 0;
    }

    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public String getTaskId() {
        return taskId;
    }

    public void setTaskId(String taskId) {
        this.taskId = taskId;
    }

    public Long getBidId() {
        return bidId;
    }

    public void setBidId(Long bidId) {
        this.bidId = bidId;
    }

    public Integer getTaskStatus() {
        return taskStatus;
    }

    public void setTaskStatus(Integer taskStatus) {
        this.taskStatus = taskStatus;
    }

    public String getAuditResult() {
        return auditResult;
    }

    public void setAuditResult(String auditResult) {
        this.auditResult = auditResult;
    }

    public Integer getIssueCount() {
        return issueCount;
    }

    public void setIssueCount(Integer issueCount) {
        this.issueCount = issueCount;
    }

    public Integer getCriticalCount() {
        return criticalCount;
    }

    public void setCriticalCount(Integer criticalCount) {
        this.criticalCount = criticalCount;
    }

    public Integer getWarningCount() {
        return warningCount;
    }

    public void setWarningCount(Integer warningCount) {
        this.warningCount = warningCount;
    }

    public Integer getInfoCount() {
        return infoCount;
    }

    public void setInfoCount(Integer infoCount) {
        this.infoCount = infoCount;
    }

    public LocalDateTime getStartTime() {
        return startTime;
    }

    public void setStartTime(LocalDateTime startTime) {
        this.startTime = startTime;
    }

    public LocalDateTime getEndTime() {
        return endTime;
    }

    public void setEndTime(LocalDateTime endTime) {
        this.endTime = endTime;
    }

    public Long getAuditUserId() {
        return auditUserId;
    }

    public void setAuditUserId(Long auditUserId) {
        this.auditUserId = auditUserId;
    }

    public LocalDateTime getCreateTime() {
        return createTime;
    }

    public void setCreateTime(LocalDateTime createTime) {
        this.createTime = createTime;
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

    public List<String> getEnabledChecks() {
        return enabledChecks;
    }

    public void setEnabledChecks(List<String> enabledChecks) {
        this.enabledChecks = enabledChecks;
    }

    public List<String> getFailedStages() {
        return failedStages;
    }

    public void setFailedStages(List<String> failedStages) {
        this.failedStages = failedStages;
    }

    public String getErrorMsg() {
        return errorMsg;
    }

    public void setErrorMsg(String errorMsg) {
        this.errorMsg = errorMsg;
    }

    public LocalDateTime getUpdatedAt() {
        return updatedAt;
    }

    public void setUpdatedAt(LocalDateTime updatedAt) {
        this.updatedAt = updatedAt;
    }

    public Long getVersion() {
        return version;
    }

    public void setVersion(Long version) {
        this.version = version;
    }
}
