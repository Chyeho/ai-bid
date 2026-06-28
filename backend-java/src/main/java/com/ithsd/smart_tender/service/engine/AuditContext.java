package com.ithsd.smart_tender.service.engine;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import com.ithsd.smart_tender.service.rag.RagChunk;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public class AuditContext {
    private String taskId;
    private Long bidId;
    private List<String> enabledChecks;
    private String stage;
    private Integer progress;
    private final List<String> failedStages;
    private final List<AuditIssueEntity> issues;
    private final Map<String, List<RagChunk>> ragResults;
    private String documentText;
    private String documentSource;
    private int issueNoCursor;

    public AuditContext() {
        this.enabledChecks = new ArrayList<>();
        this.progress = 0;
        this.failedStages = new ArrayList<>();
        this.issues = new ArrayList<>();
        this.ragResults = new LinkedHashMap<>();
        this.issueNoCursor = 0;
    }

    public static AuditContext fromTask(String taskId, Long bidId, List<String> enabledChecks) {
        AuditContext context = new AuditContext();
        context.taskId = taskId;
        context.bidId = bidId;
        if (enabledChecks == null || enabledChecks.isEmpty()) {
            context.enabledChecks = new ArrayList<>(AuditCheckTypeEnum.valuesSet());
        } else {
            context.enabledChecks = new ArrayList<>(enabledChecks);
        }
        return context;
    }

    public void addFailedStage(String stageName) {
        if (!failedStages.contains(stageName)) {
            failedStages.add(stageName);
        }
    }

    public AuditIssueEntity addIssue(AuditIssueEntity issue) {
        if (issue == null) {
            return null;
        }
        issueNoCursor++;
        issue.setIssueNo(String.format("%06d", issueNoCursor));
        issues.add(issue);
        return issue;
    }

    public List<AuditIssueEntity> addIssues(List<AuditIssueEntity> newIssues) {
        List<AuditIssueEntity> added = new ArrayList<>();
        if (newIssues == null || newIssues.isEmpty()) {
            return added;
        }
        for (AuditIssueEntity issue : newIssues) {
            AuditIssueEntity addedIssue = addIssue(issue);
            if (addedIssue != null) {
                added.add(addedIssue);
            }
        }
        return added;
    }

    public void increaseProgress(Integer delta) {
        int value = this.progress == null ? 0 : this.progress;
        int step = delta == null ? 0 : delta;
        this.progress = Math.min(100, value + Math.max(step, 0));
    }

    public void putRagChunks(String checkType, List<RagChunk> chunks) {
        if (checkType == null || checkType.isBlank()) {
            return;
        }
        if (chunks == null || chunks.isEmpty()) {
            return;
        }
        ragResults.put(checkType, new ArrayList<>(chunks));
    }

    public List<RagChunk> getRagChunks(String checkType) {
        List<RagChunk> chunks = ragResults.get(checkType);
        if (chunks == null) {
            return List.of();
        }
        return chunks;
    }

    public String getTaskId() {
        return taskId;
    }

    public Long getBidId() {
        return bidId;
    }

    public List<String> getEnabledChecks() {
        return enabledChecks;
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

    public List<String> getFailedStages() {
        return failedStages;
    }

    public List<AuditIssueEntity> getIssues() {
        return issues;
    }

    public Map<String, List<RagChunk>> getRagResults() {
        return ragResults;
    }

    public String getDocumentText() {
        return documentText;
    }

    public void setDocumentText(String documentText) {
        this.documentText = documentText;
    }

    public String getDocumentSource() {
        return documentSource;
    }

    public void setDocumentSource(String documentSource) {
        this.documentSource = documentSource;
    }
}
