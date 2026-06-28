package com.ithsd.smart_tender.service.engine.check;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.service.engine.AuditContext;
import com.ithsd.smart_tender.service.llm.JsonLinesIssueParser;
import com.ithsd.smart_tender.service.llm.LlmClient;
import com.ithsd.smart_tender.service.llm.PromptLoader;
import com.ithsd.smart_tender.service.rag.RagChunk;
import org.springframework.util.StringUtils;

import java.util.ArrayList;
import java.util.List;

public abstract class AbstractLlmAuditCheck {
    private static final String DEFAULT_PROMPT_VERSION = "v1";
    private final PromptLoader promptLoader;
    private final LlmClient llmClient;
    private final JsonLinesIssueParser issueParser;

    protected AbstractLlmAuditCheck(PromptLoader promptLoader, LlmClient llmClient, JsonLinesIssueParser issueParser) {
        this.promptLoader = promptLoader;
        this.llmClient = llmClient;
        this.issueParser = issueParser;
    }

    protected List<AuditIssueEntity> runLlm(AuditContext context, String checkType) {
        String promptTemplate = promptLoader.load(checkType, DEFAULT_PROMPT_VERSION);
        List<RagChunk> ragChunks = context.getRagChunks(checkType);
        String prompt = promptTemplate
                + "\ntaskId=" + context.getTaskId()
                + "\nbidId=" + context.getBidId()
                + "\ndocumentSource=" + (context.getDocumentSource() == null ? "none" : context.getDocumentSource())
                + "\ndocumentText:\n"
                + buildDocumentPrompt(context.getDocumentText())
                + "\nragContext:\n"
                + buildRagPrompt(ragChunks);
        String jsonLines = llmClient.complete(checkType, prompt);
        List<AuditIssueEntity> issues = issueParser.parse(checkType, jsonLines);
        if (!ragChunks.isEmpty()) {
            RagChunk first = ragChunks.get(0);
            for (AuditIssueEntity issue : issues) {
                if (!StringUtils.hasText(issue.getReference()) || "stub_llm".equals(issue.getReference()) || "stub_llm_v1".equals(issue.getReference())) {
                    issue.setReference(first.getReference());
                }
                if (!StringUtils.hasText(issue.getSectionName())) {
                    issue.setSectionName(first.getSectionName());
                }
            }
        }
        return issues;
    }

    private String buildRagPrompt(List<RagChunk> ragChunks) {
        if (ragChunks == null || ragChunks.isEmpty()) {
            return "none";
        }
        List<String> lines = new ArrayList<>();
        for (RagChunk chunk : ragChunks) {
            String line = "[reference=" + chunk.getReference()
                    + ", section=" + chunk.getSectionName()
                    + "] " + chunk.getContent();
            lines.add(line);
        }
        return String.join("\n", lines);
    }

    private String buildDocumentPrompt(String documentText) {
        if (!StringUtils.hasText(documentText)) {
            return "none";
        }
        String normalized = documentText.replace("\r", "\n");
        if (normalized.length() <= 3000) {
            return normalized;
        }
        return normalized.substring(0, 3000);
    }
}
