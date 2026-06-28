package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.pojo.dto.CreateDocumentParseJobRequest;
import com.ithsd.smart_tender.pojo.vo.CreateDocumentParseJobVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseChunkPageVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseJobStatusVO;

public interface DocumentParseJobService {
    CreateDocumentParseJobVO createJob(CreateDocumentParseJobRequest request);

    DocumentParseJobStatusVO getStatus(String jobId);

    DocumentParseChunkPageVO listChunks(String jobId, Integer page, Integer size, String sinceChunkId);
}
