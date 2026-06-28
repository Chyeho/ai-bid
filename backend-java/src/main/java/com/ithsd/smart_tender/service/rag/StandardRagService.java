package com.ithsd.smart_tender.service.rag;

import com.ithsd.smart_tender.service.engine.AuditContext;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.stereotype.Service;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

@Service
public class StandardRagService {
    private static final Logger log = LoggerFactory.getLogger(StandardRagService.class);
    private final Retriever retriever;
    private final Retriever stubRetriever;
    private final RagContextMerger ragContextMerger;
    private final AuditRagProperties ragProperties;

    public StandardRagService(@Qualifier("ragRetriever") Retriever retriever, @Qualifier("stubRetriever") Retriever stubRetriever, RagContextMerger ragContextMerger, AuditRagProperties ragProperties) {
        this.retriever = retriever;
        this.stubRetriever = stubRetriever;
        this.ragContextMerger = ragContextMerger;
        this.ragProperties = ragProperties;
    }

    public Map<String, List<RagChunk>> retrieve(AuditContext context) {
        Map<String, List<RagChunk>> results = new LinkedHashMap<>();
        if (context == null || !ragProperties.isEnabled()) {
            return results;
        }
        for (String checkType : context.getEnabledChecks()) {
            try {
                List<RagChunk> chunks = retriever.retrieve(context.getBidId(), checkType, ragProperties.getTopK());
                if (chunks != null && !chunks.isEmpty()) {
                    results.put(checkType, chunks);
                }
            } catch (RuntimeException ex) {
                context.addFailedStage("RAG_" + checkType.toUpperCase());
                log.warn("retrieve rag failed, taskId={}, checkType={}", context.getTaskId(), checkType, ex);
                if (ragProperties.isFallbackToStub() && !"stub".equalsIgnoreCase(ragProperties.getProvider())) {
                    try {
                        List<RagChunk> chunks = stubRetriever.retrieve(context.getBidId(), checkType, ragProperties.getTopK());
                        if (chunks != null && !chunks.isEmpty()) {
                            results.put(checkType, chunks);
                        }
                    } catch (RuntimeException fallbackEx) {
                        log.warn("fallback stub rag failed, taskId={}, checkType={}", context.getTaskId(), checkType, fallbackEx);
                    }
                }
            }
        }
        return results;
    }

    public void retrieveAndMerge(AuditContext context) {
        ragContextMerger.merge(context, retrieve(context));
    }
}
