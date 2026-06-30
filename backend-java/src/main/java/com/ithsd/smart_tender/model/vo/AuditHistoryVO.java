package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import java.io.Serializable;
import java.math.BigDecimal;
import java.time.LocalDateTime;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class AuditHistoryVO implements Serializable {
    private Long id;
    private String taskId;
    private Long bidId;
    private String projectName;
    private String fileCategory;
    private String supplierName;
    private BigDecimal budgetAmount;
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
}
