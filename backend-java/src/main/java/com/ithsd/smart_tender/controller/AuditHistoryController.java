package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.dto.AuditHistoryPageQueryDTO;
import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.AuditHistoryDetailVO;
import com.ithsd.smart_tender.service.AuditHistoryService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;

import java.util.Map;

@RestController
@RequestMapping("/api/audit-history")
@RequiredArgsConstructor
public class AuditHistoryController {

    private final AuditHistoryService auditHistoryService;

    @GetMapping
    public Result<PageResult> page(AuditHistoryPageQueryDTO dto) {
        PageResult pageResult = auditHistoryService.page(dto);
        return Result.success(pageResult);
    }

    @GetMapping("/{auditId}")
    public Result<AuditHistoryDetailVO> getDetailById(@PathVariable Long auditId) {
        AuditHistoryDetailVO vo = auditHistoryService.getDetailById(auditId);
        return Result.success(vo);
    }

    @GetMapping("/statistics")
    public Result<Map<String, Object>> getStatistics(AuditHistoryPageQueryDTO dto) {
        Map<String, Object> statistics = auditHistoryService.getStatistics(dto);
        return Result.success(statistics);
    }

    @DeleteMapping("/{auditId}")
    public Result delete(@PathVariable Long auditId) {
        auditHistoryService.delete(auditId);
        return Result.success();
    }
}
