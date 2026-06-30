package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;
import java.time.LocalDateTime;
import java.util.List;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ProjectVO implements Serializable {
    private Long id;
    private Long userId;
    private String projectName;
    private String supplierName;
    private Integer parseStatus; // 0未审核 1已审核
    private Integer latestVersion;
    private LocalDateTime createTime;
    private LocalDateTime updateTime;
    private String fileCategory;
    private String auditResult;

    // 该项目下的标书及其审核报告列表
    private List<TenderWithAuditVO> tenders;
}
