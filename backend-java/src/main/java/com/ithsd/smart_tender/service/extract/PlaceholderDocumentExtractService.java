package com.ithsd.smart_tender.service.extract;

import com.ithsd.smart_tender.exception.BizException;
import org.springframework.stereotype.Component;

@Component("placeholderDocumentExtractService")
public class PlaceholderDocumentExtractService implements DocumentExtractService {
    @Override
    public ExtractedDocument extract(Long bidId) {
        throw new BizException(5604, "DOC_EXTRACT_PLACEHOLDER_NOT_READY");
    }
}
