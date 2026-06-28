package com.ithsd.smart_tender.pojo.vo;

import com.ithsd.smart_tender.pojo.entity.AuditReport;
import com.ithsd.smart_tender.pojo.entity.AuditTask;
import com.ithsd.smart_tender.pojo.entity.Tender;
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
