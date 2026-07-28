package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import java.io.Serializable;
import java.time.LocalDateTime;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class AuditIssueVO implements Serializable {
    private Long id;
    private Long auditId;
    private String issueNo;
    private String severity;
    private Boolean isCritical;
    private String criticalReason;
    private String category;
    private String description;
    private String suggestion;
    private Integer pageNumber;
    private String sectionName;
    private String context;
    private String reference;
    private LocalDateTime createTime;
}
