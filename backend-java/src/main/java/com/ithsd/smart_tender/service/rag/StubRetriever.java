package com.ithsd.smart_tender.service.rag;

import org.springframework.stereotype.Component;

import java.util.ArrayList;
import java.util.List;

@Component("stubRetriever")
public class StubRetriever implements Retriever {
    @Override
    public List<RagChunk> retrieve(Long bidId, String checkType, int topK) {
        List<RagChunk> chunks = new ArrayList<>();
        int limit = Math.max(topK, 1);
        for (int i = 1; i <= limit; i++) {
            RagChunk chunk = new RagChunk();
            chunk.setCheckType(checkType);
            chunk.setReference("STD-" + checkType.toUpperCase() + "-" + String.format("%03d", i));
            chunk.setSectionName(checkType + "_section_" + i);
            chunk.setContent("bidId=" + bidId + ", checkType=" + checkType + ", chunkIndex=" + i);
            chunks.add(chunk);
        }
        return chunks;
    }
}
