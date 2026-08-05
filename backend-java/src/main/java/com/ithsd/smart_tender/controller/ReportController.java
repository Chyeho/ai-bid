package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.ReportVO;
import com.ithsd.smart_tender.service.ReportService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping("/api/audit-reports")
@RequiredArgsConstructor
public class ReportController {

    private final ReportService reportService;

    @PostMapping("/{taskIdOrAuditId}/generate")
    public Result<ReportVO> generateReport(@PathVariable String taskIdOrAuditId) {
        ReportVO vo = reportService.generateReport(taskIdOrAuditId);
        return Result.success(vo);
    }

    @GetMapping("/{taskIdOrAuditId}")
    public Result<ReportVO> getReport(@PathVariable String taskIdOrAuditId) {
        String content = reportService.getReportContent(taskIdOrAuditId);
        if (content == null) {
            return Result.error("报告不存在");
        }
        Long resolvedAuditId;
        try {
            resolvedAuditId = Long.parseLong(taskIdOrAuditId);
        } catch (NumberFormatException ex) {
            resolvedAuditId = null;
        }
        ReportVO vo = ReportVO.builder()
                .auditId(resolvedAuditId)
                .docContent(content)
                .build();
        return Result.success(vo);
    }
}
