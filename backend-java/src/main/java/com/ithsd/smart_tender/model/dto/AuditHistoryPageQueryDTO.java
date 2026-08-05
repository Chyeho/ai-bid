package com.ithsd.smart_tender.model.dto;

import lombok.Data;
import java.io.Serializable;
import java.time.LocalDate;

@Data
public class AuditHistoryPageQueryDTO implements Serializable {
    private int page = 1;
    private int size = 10;
    private String projectName;
    private String fileCategory;
    private String auditResult;
    private Long auditUserId;
    private LocalDate startDate;
    private LocalDate endDate;
}
