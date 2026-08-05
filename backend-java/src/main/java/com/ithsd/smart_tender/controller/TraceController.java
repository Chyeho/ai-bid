package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.TraceSessionDetailVO;
import com.ithsd.smart_tender.model.vo.TraceSessionVO;
import com.ithsd.smart_tender.service.TraceService;
import org.springframework.validation.annotation.Validated;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

/**
 * 审查追溯 REST API — 设计文档 §10.1.5。
 */
@Validated
@RestController
@RequestMapping("/api")
public class TraceController {

    private final TraceService traceService;

    public TraceController(TraceService traceService) {
        this.traceService = traceService;
    }

    @GetMapping("/audit-tasks/{taskId}/traces")
    public Result listSessions(
            @PathVariable String taskId,
            @RequestParam(required = false) String agent,
            @RequestParam(required = false) String severity,
            @RequestParam(defaultValue = "1") int page,
            @RequestParam(defaultValue = "20") int size) {
        PageResult result = traceService.listByTaskId(taskId, agent, severity, page, size);
        return Result.success(result);
    }

    @GetMapping("/traces/{sessionId}")
    public Result getSessionDetail(@PathVariable String sessionId) {
        TraceSessionDetailVO detail = traceService.getSessionDetail(sessionId);
        if (detail == null) {
            return Result.error("追溯会话不存在");
        }
        return Result.success(detail);
    }
}
