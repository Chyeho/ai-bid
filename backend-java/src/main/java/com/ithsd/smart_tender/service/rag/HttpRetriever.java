package com.ithsd.smart_tender.service.rag;

import com.ithsd.smart_tender.exception.BizException;
import org.springframework.stereotype.Component;

import java.util.List;

@Component("httpRetriever")
public class HttpRetriever implements Retriever {
    @Override
    public List<RagChunk> retrieve(Long bidId, String checkType, int topK) {
        throw new BizException(5601, "RAG_HTTP_NOT_READY");
    }
}
