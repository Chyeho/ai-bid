package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.dto.AuditHistoryPageQueryDTO;
import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.vo.AuditHistoryDetailVO;

import java.util.Map;

public interface AuditHistoryService {
    PageResult page(AuditHistoryPageQueryDTO dto);
    
    AuditHistoryDetailVO getDetailById(Long id);
    
    void delete(Long id);
    
    Map<String, Object> getStatistics(AuditHistoryPageQueryDTO dto);
}
