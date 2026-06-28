package com.ithsd.smart_tender.service.rag;

import com.ithsd.smart_tender.exception.BizException;
import org.springframework.context.annotation.Primary;
import org.springframework.stereotype.Component;

import java.util.List;

@Primary
@Component("ragRetriever")
public class SwitchableRetriever implements Retriever {
    private final StubRetriever stubRetriever;
    private final HttpRetriever httpRetriever;
    private final AuditRagProperties ragProperties;

    public SwitchableRetriever(StubRetriever stubRetriever, HttpRetriever httpRetriever, AuditRagProperties ragProperties) {
        this.stubRetriever = stubRetriever;
        this.httpRetriever = httpRetriever;
        this.ragProperties = ragProperties;
    }

    @Override
    public List<RagChunk> retrieve(Long bidId, String checkType, int topK) {
        if ("http".equalsIgnoreCase(ragProperties.getProvider())) {
            if (!ragProperties.isHttpEnabled()) {
                throw new BizException(5602, "RAG_HTTP_DISABLED");
            }
            return httpRetriever.retrieve(bidId, checkType, topK);
        }
        return stubRetriever.retrieve(bidId, checkType, topK);
    }
}
