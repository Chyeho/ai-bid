package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import java.io.Serializable;
import java.math.BigDecimal;
import java.time.LocalDateTime;
import java.util.List;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class AuditHistoryDetailVO implements Serializable {
    private Long id;
    private String taskId;
    private Long bidId;
    
    private String fileName;
    private String fileType;
    private String projectName;
    private String fileCategory;
    private String supplierName;
    private BigDecimal budgetAmount;
    private Integer pageCount;
    
    private Integer taskStatus;
    private String auditResult;
    private Integer issueCount;
    private Integer criticalCount;
    private Integer warningCount;
    private Integer infoCount;
    private LocalDateTime startTime;
    private LocalDateTime endTime;
    private String auditUserName;
    private LocalDateTime createTime;
    
    private List<AuditIssueVO> issues;
    
    private String docContent;
    private LocalDateTime reportGenerateTime;
}
