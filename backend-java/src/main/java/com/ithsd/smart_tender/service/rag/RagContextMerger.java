package com.ithsd.smart_tender.service.rag;

import com.ithsd.smart_tender.service.engine.AuditContext;
import org.springframework.stereotype.Component;

import java.util.List;
import java.util.Map;

@Component
public class RagContextMerger {
    public void merge(AuditContext context, Map<String, List<RagChunk>> ragResults) {
        if (context == null || ragResults == null || ragResults.isEmpty()) {
            return;
        }
        for (Map.Entry<String, List<RagChunk>> entry : ragResults.entrySet()) {
            context.putRagChunks(entry.getKey(), entry.getValue());
        }
    }
}
