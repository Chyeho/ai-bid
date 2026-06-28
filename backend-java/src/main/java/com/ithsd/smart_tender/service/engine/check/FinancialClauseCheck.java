package com.ithsd.smart_tender.service.engine.check;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import com.ithsd.smart_tender.pojo.enums.AuditStageEnum;
import com.ithsd.smart_tender.service.engine.AuditCheck;
import com.ithsd.smart_tender.service.engine.AuditContext;
import com.ithsd.smart_tender.service.llm.JsonLinesIssueParser;
import com.ithsd.smart_tender.service.llm.LlmClient;
import com.ithsd.smart_tender.service.llm.PromptLoader;

import java.util.List;

public class FinancialClauseCheck extends AbstractLlmAuditCheck implements AuditCheck {
    public FinancialClauseCheck(PromptLoader promptLoader, LlmClient llmClient, JsonLinesIssueParser issueParser) {
        super(promptLoader, llmClient, issueParser);
    }

    @Override
    public String getCheckType() {
        return AuditCheckTypeEnum.DEMAND.getValue();
    }

    @Override
    public AuditStageEnum getStage() {
        return AuditStageEnum.CHECK_DEMAND;
    }

    @Override
    public List<AuditIssueEntity> check(AuditContext context) {
        return runLlm(context, getCheckType());
    }
}
