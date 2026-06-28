package com.ithsd.smart_tender.service.rag;

import java.util.List;

public interface Retriever {
    List<RagChunk> retrieve(Long bidId, String checkType, int topK);
}
