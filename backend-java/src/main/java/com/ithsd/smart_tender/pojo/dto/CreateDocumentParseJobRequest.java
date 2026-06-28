package com.ithsd.smart_tender.pojo.dto;

import jakarta.validation.constraints.NotBlank;

public class CreateDocumentParseJobRequest {
    @NotBlank(message = "fileId不能为空")
    private String fileId;
    private String fileName;
    @NotBlank(message = "sourceType不能为空")
    private String sourceType;
    private String priority = "normal";
    private Boolean triggerRag = Boolean.TRUE;
    private String strategyVersion = "chunk-v1";
    private String requestId;

    public String getFileId() {
        return fileId;
    }

    public void setFileId(String fileId) {
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

    public String getRequestId() {
        return requestId;
    }

    public void setRequestId(String requestId) {
        this.requestId = requestId;
    }
}
