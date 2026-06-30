package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.service.AuditIssueService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.Mapping;
import org.springframework.web.bind.annotation.RequestMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * @ Author：YangYu
 * @ Package：com.ithsd.smart_tender.controller
 * @ Project：smart_tender
 * @ Description:
 * @ Date：2026/3/11  10:50
 */

@RestController
@RequestMapping("/api/audit-issues")
@RequiredArgsConstructor
public class AuditIssueController {

    private final AuditIssueService auditIssueService;

    /**
     * 统计个人审核问题类别
     * @return
     */
    @GetMapping("/count-issue")
    public Result<Map<String, Long>> countByCategory() {
        return Result.success(auditIssueService.countByCategory());
    }


}
