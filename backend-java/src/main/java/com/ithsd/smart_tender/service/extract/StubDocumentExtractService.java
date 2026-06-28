package com.ithsd.smart_tender.service.extract;

import org.springframework.stereotype.Component;

@Component("stubDocumentExtractService")
public class StubDocumentExtractService implements DocumentExtractService {
    @Override
    public ExtractedDocument extract(Long bidId) {
        ExtractedDocument document = new ExtractedDocument();
        document.setBidId(bidId);
        document.setSource("stub");
        document.setContent("stub extracted content for bidId=" + bidId);
        document.setDegraded(false);
        return document;
    }
}
