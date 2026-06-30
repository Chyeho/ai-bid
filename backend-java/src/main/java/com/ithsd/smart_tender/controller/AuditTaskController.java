package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.dto.CreateAuditTaskRequest;
import com.ithsd.smart_tender.model.dto.rust.RustBlockBBoxResponse;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.AuditTaskCreateVO;
import com.ithsd.smart_tender.model.vo.AuditTaskStatusVO;
import com.ithsd.smart_tender.model.vo.ResultVO;
import com.ithsd.smart_tender.service.AuditTaskService;
import jakarta.validation.Valid;
import jakarta.validation.constraints.Min;
import jakarta.validation.constraints.NotBlank;
import org.springframework.http.MediaType;
import org.springframework.validation.annotation.Validated;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestHeader;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter;

import java.util.List;
import java.util.Map;

@Validated
@RestController
@RequestMapping("/api/audit-tasks")
public class AuditTaskController {
    private final AuditTaskService auditTaskService;

    public AuditTaskController(AuditTaskService auditTaskService) {
        this.auditTaskService = auditTaskService;
    }

    @PostMapping
    public Result<AuditTaskCreateVO> createTask(@Valid @RequestBody CreateAuditTaskRequest request) {
        return Result.success(auditTaskService.createTask(request));
    }

    @GetMapping("/{taskId}")
    public Result<AuditTaskStatusVO> getStatus(@PathVariable @NotBlank(message = "taskId不能为空") String taskId) {
        return Result.success(auditTaskService.getStatus(taskId));
    }

    @GetMapping("/{taskId}/result")
    public Result<ResultVO> getResult(
            @PathVariable @NotBlank(message = "taskId不能为空") String taskId,
            @RequestParam(value = "page", required = false, defaultValue = "1") @Min(value = 1, message = "page必须从1开始") Integer page,
            @RequestParam(value = "size", required = false, defaultValue = "20") @Min(value = 1, message = "size必须大于0") Integer size,
            @RequestParam(value = "sinceIssueNo", required = false) String sinceIssueNo
    ) {
        return Result.success(auditTaskService.getResult(taskId, page, size, sinceIssueNo));
    }

    @GetMapping(value = "/{taskId}/stream", produces = MediaType.TEXT_EVENT_STREAM_VALUE)
    public SseEmitter stream(
            @PathVariable @NotBlank(message = "taskId不能为空") String taskId,
            @RequestHeader(value = "Last-Event-ID", required = false) String lastEventId
    ) {
        return auditTaskService.subscribeStream(taskId, lastEventId);
    }

    /**
     * 统计用户本周审核数（周一到周日）
     * @return
     */
    @GetMapping("/count-audit")
    public Result<Map<String, Long>> countByWeek() {
        return Result.success(auditTaskService.countByWeek());
    }

    @PostMapping("/callback")
    public Result<Void> callback(
            @RequestParam("taskId") String taskId,
            @RequestBody String responseBody
    ) {
        auditTaskService.processAuditResult(taskId, responseBody);
        return Result.success();
    }

    /**
     * 查询指定 block_id 的 BBox 坐标（代理到 Rust 引擎）。
     * 前端用于 bbox-based PDF 精确高亮。
     */
    @GetMapping("/{taskId}/blocks")
    public Result<List<RustBlockBBoxResponse>> getBlockBboxes(
            @PathVariable @NotBlank(message = "taskId不能为空") String taskId,
            @RequestParam("ids") String blockIds
    ) {
        return Result.success(auditTaskService.getBlockBboxes(taskId, blockIds));
    }

}
