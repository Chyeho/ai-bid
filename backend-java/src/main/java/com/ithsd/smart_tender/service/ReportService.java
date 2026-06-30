package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.vo.ReportVO;

public interface ReportService {
    ReportVO generateReport(String auditIdOrTaskId);
    
    String getReportContent(String auditIdOrTaskId);
}
