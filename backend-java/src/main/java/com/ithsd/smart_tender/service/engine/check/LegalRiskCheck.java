package com.ithsd.smart_tender.service.engine.check;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import com.ithsd.smart_tender.pojo.enums.AuditStageEnum;
import com.ithsd.smart_tender.service.engine.AuditCheck;
import com.ithsd.smart_tender.service.engine.AuditContext;
import com.ithsd.smart_tender.service.llm.JsonLinesIssueParser;
import com.ithsd.smart_tender.service.llm.LlmClient;
import com.ithsd.smart_tender.service.llm.PromptLoader;
import org.springframework.stereotype.Component;

import java.util.List;

@Component
public class LegalRiskCheck extends AbstractLlmAuditCheck implements AuditCheck {
    public LegalRiskCheck(PromptLoader promptLoader, LlmClient llmClient, JsonLinesIssueParser issueParser) {
        super(promptLoader, llmClient, issueParser);
    }

    @Override
    public String getCheckType() {
        return AuditCheckTypeEnum.LEGAL.getValue();
    }

    @Override
    public AuditStageEnum getStage() {
        return AuditStageEnum.CHECK_LEGAL;
    }

    @Override
    public List<AuditIssueEntity> check(AuditContext context) {
        return runLlm(context, getCheckType());
    }
}
