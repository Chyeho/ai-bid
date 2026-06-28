package com.ithsd.smart_tender.service.engine;

import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "audit.progress")
public class AuditProgressProperties {
    private Integer docExtract = 10;
    private Integer rag = 15;
    private Integer budget = 25;
    private Integer demand = 25;
    private Integer legal = 25;

    public int beforeChecksProgress() {
        return safe(docExtract) + safe(rag);
    }

    public int checkWeight(String checkType) {
        if (AuditCheckTypeEnum.BUDGET.getValue().equals(checkType)) {
            return safe(budget);
        }
        if (AuditCheckTypeEnum.DEMAND.getValue().equals(checkType)) {
            return safe(demand);
        }
        if (AuditCheckTypeEnum.LEGAL.getValue().equals(checkType)) {
            return safe(legal);
        }
        return 0;
    }

    private int safe(Integer value) {
        if (value == null || value < 0) {
            return 0;
        }
        return value;
    }

    public Integer getDocExtract() {
        return docExtract;
    }

    public void setDocExtract(Integer docExtract) {
        this.docExtract = docExtract;
    }

    public Integer getRag() {
        return rag;
    }

    public void setRag(Integer rag) {
        this.rag = rag;
    }

    public Integer getBudget() {
        return budget;
    }

    public void setBudget(Integer budget) {
        this.budget = budget;
    }

    public Integer getDemand() {
        return demand;
    }

    public void setDemand(Integer demand) {
        this.demand = demand;
    }

    public Integer getLegal() {
        return legal;
    }

    public void setLegal(Integer legal) {
        this.legal = legal;
    }
}
