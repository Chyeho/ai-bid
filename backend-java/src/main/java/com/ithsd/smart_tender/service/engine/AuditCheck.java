package com.ithsd.smart_tender.service.engine;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.enums.AuditStageEnum;

import java.util.List;

public interface AuditCheck {
    String getCheckType();

    AuditStageEnum getStage();

    List<AuditIssueEntity> check(AuditContext context);
}
