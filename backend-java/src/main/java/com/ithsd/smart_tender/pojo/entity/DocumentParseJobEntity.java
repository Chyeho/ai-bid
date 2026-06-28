package com.ithsd.smart_tender.pojo.entity;

import com.ithsd.smart_tender.repository.converter.StringListJsonConverter;
import jakarta.persistence.Column;
import jakarta.persistence.Convert;
import jakarta.persistence.Entity;
import jakarta.persistence.GeneratedValue;
import jakarta.persistence.GenerationType;
import jakarta.persistence.Id;
import jakarta.persistence.Index;
import jakarta.persistence.Table;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;

@Entity
@Table(name = "document_parse_job", indexes = {
        @Index(name = "uk_document_parse_job_job_id", columnList = "job_id", unique = true),
        @Index(name = "idx_document_parse_job_file_id", columnList = "file_id"),
        @Index(name = "idx_document_parse_job_status", columnList = "status"),
        @Index(name = "idx_document_parse_job_created_at", columnList = "created_at"),
        @Index(name = "idx_document_parse_job_request_id", columnList = "request_id")
})
public class DocumentParseJobEntity {
    @Id
    @GeneratedValue(strategy = GenerationType.IDENTITY)
    private Long id;

    @Column(name = "job_id", nullable = false, unique = true, length = 64)
    private String jobId;

    @Column(name = "request_id", length = 64)
    private String requestId;

    @Column(name = "file_id", nullable = false)
    private Long fileId;

    @Column(name = "file_name", length = 255)
    private String fileName;

    @Column(name = "source_type", nullable = false, length = 16)
    private String sourceType;

    @Column(name = "priority", nullable = false, length = 16)
    private String priority;

    @Column(name = "trigger_rag", nullable = false)
    private Boolean triggerRag;

    @Column(name = "strategy_version", nullable = false, length = 64)
    private String strategyVersion;

    @Column(name = "status", nullable = false, length = 24)
    private String status;

    @Column(name = "stage", length = 64)
    private String stage;

    @Column(name = "progress", nullable = false)
    private Integer progress;

    @Column(name = "chunk_count", nullable = false)
    private Integer chunkCount;

    @Convert(converter = StringListJsonConverter.class)
    @Column(name = "failed_stages", columnDefinition = "json")
    private List<String> failedStages;

    @Column(name = "error_msg", length = 1000)
    private String errorMsg;

    @Column(name = "start_time")
    private LocalDateTime startTime;

    @Column(name = "end_time")
    private LocalDateTime endTime;

    @Column(name = "created_at", nullable = false)
    private LocalDateTime createdAt;

    @Column(name = "updated_at", nullable = false)
    private LocalDateTime updatedAt;

    public DocumentParseJobEntity() {
        this.failedStages = new ArrayList<>();
        this.progress = 0;
        this.chunkCount = 0;
        this.triggerRag = Boolean.TRUE;
    }

    public Long getId() {
        return id;
    }

    public void setId(Long id) {
        this.id = id;
    }

    public String getJobId() {
        return jobId;
    }

    public void setJobId(String jobId) {
        this.jobId = jobId;
    }

    public String getRequestId() {
        return requestId;
    }

    public void setRequestId(String requestId) {
        this.requestId = requestId;
    }

    public Long getFileId() {
        return fileId;
    }

    public void setFileId(Long fileId) {
        this.fileId = fileId;
    }

    public String getFileName() {
        return fileName;
    }

    public void setFileName(String fileName) {
        this.fileName = fileName;
    }

    public String getSourceType() {
        return sourceType;
    }

    public void setSourceType(String sourceType) {
        this.sourceType = sourceType;
    }

    public String getPriority() {
        return priority;
    }

    public void setPriority(String priority) {
        this.priority = priority;
    }

    public Boolean getTriggerRag() {
        return triggerRag;
    }

    public void setTriggerRag(Boolean triggerRag) {
        this.triggerRag = triggerRag;
    }

    public String getStrategyVersion() {
        return strategyVersion;
    }

    public void setStrategyVersion(String strategyVersion) {
        this.strategyVersion = strategyVersion;
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

    public Integer getChunkCount() {
        return chunkCount;
    }

    public void setChunkCount(Integer chunkCount) {
        this.chunkCount = chunkCount;
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

    public LocalDateTime getCreatedAt() {
        return createdAt;
    }

    public void setCreatedAt(LocalDateTime createdAt) {
        this.createdAt = createdAt;
    }

    public LocalDateTime getUpdatedAt() {
        return updatedAt;
    }

    public void setUpdatedAt(LocalDateTime updatedAt) {
        this.updatedAt = updatedAt;
    }
}
