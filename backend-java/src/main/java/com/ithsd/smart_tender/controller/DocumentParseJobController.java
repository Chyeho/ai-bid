package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.pojo.dto.CreateDocumentParseJobRequest;
import com.ithsd.smart_tender.pojo.result.Result;
import com.ithsd.smart_tender.pojo.vo.CreateDocumentParseJobVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseChunkPageVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseJobStatusVO;
import com.ithsd.smart_tender.service.DocumentParseJobService;
import jakarta.validation.Valid;
import jakarta.validation.constraints.Min;
import jakarta.validation.constraints.NotBlank;
import org.springframework.validation.annotation.Validated;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.PathVariable;
import org.springframework.web.bind.annotation.PostMapping;
import org.springframework.web.bind.annotation.RequestBody;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RequestParam;
import org.springframework.web.bind.annotation.RestController;

@Validated
@RestController
@RequestMapping("/api/document-parse-jobs")
public class DocumentParseJobController {
    private final DocumentParseJobService documentParseJobService;

    public DocumentParseJobController(DocumentParseJobService documentParseJobService) {
        this.documentParseJobService = documentParseJobService;
    }

    @PostMapping
    public Result<CreateDocumentParseJobVO> create(@Valid @RequestBody CreateDocumentParseJobRequest request) {
        return Result.success(documentParseJobService.createJob(request));
    }

    @GetMapping("/{jobId}")
    public Result<DocumentParseJobStatusVO> getStatus(@PathVariable @NotBlank(message = "jobId不能为空") String jobId) {
        return Result.success(documentParseJobService.getStatus(jobId));
    }

    @GetMapping("/{jobId}/chunks")
    public Result<DocumentParseChunkPageVO> listChunks(
            @PathVariable @NotBlank(message = "jobId不能为空") String jobId,
            @RequestParam(value = "page", required = false, defaultValue = "1") @Min(value = 1, message = "page必须从1开始") Integer page,
            @RequestParam(value = "size", required = false, defaultValue = "20") @Min(value = 1, message = "size必须大于0") Integer size,
            @RequestParam(value = "sinceChunkId", required = false) String sinceChunkId
    ) {
        return Result.success(documentParseJobService.listChunks(jobId, page, size, sinceChunkId));
    }
}
