package com.ithsd.smart_tender.model.vo;

import com.ithsd.smart_tender.model.entity.AuditReport;
import com.ithsd.smart_tender.model.entity.AuditTask;
import com.ithsd.smart_tender.model.entity.Tender;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class TenderWithAuditVO implements Serializable {
    private Tender tender;
    private AuditTask auditTask;
    private AuditReport auditReport;
}
