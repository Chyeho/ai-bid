package com.ithsd.smart_tender.pojo.dto;

import com.ithsd.smart_tender.pojo.enums.AuditCheckTypeEnum;
import jakarta.validation.constraints.AssertTrue;
import jakarta.validation.constraints.NotNull;
import jakarta.validation.constraints.Positive;

import java.util.Arrays;
import java.util.List;
import java.util.Set;

public class CreateAuditTaskRequest {
    @NotNull(message = "bidId不能为空")
    @Positive(message = "bidId必须为正整数")
    private Long bidId;

    private List<String> enabledChecks;
    private Boolean webSearchEnabled;
    private Boolean forceRefresh;

    public Long getBidId() {
        return bidId;
    }

    public void setBidId(Long bidId) {
        this.bidId = bidId;
    }

    public List<String> getEnabledChecks() {
        return enabledChecks;
    }

    public void setEnabledChecks(List<String> enabledChecks) {
        this.enabledChecks = enabledChecks;
    }

    public List<String> enabledChecksOrDefault() {
        if (enabledChecks == null || enabledChecks.isEmpty()) {
            return Arrays.stream(AuditCheckTypeEnum.values()).map(AuditCheckTypeEnum::getValue).toList();
        }
        return enabledChecks;
    }

    public Boolean getWebSearchEnabled() {
        return webSearchEnabled;
    }

    public void setWebSearchEnabled(Boolean webSearchEnabled) {
        this.webSearchEnabled = webSearchEnabled;
    }

    public Boolean getForceRefresh() {
        return forceRefresh;
    }

    public void setForceRefresh(Boolean forceRefresh) {
        this.forceRefresh = forceRefresh;
    }

    @AssertTrue(message = "enabledChecks存在不支持的值")
    public boolean isEnabledChecksValid() {
        if (enabledChecks == null || enabledChecks.isEmpty()) {
            return true;
        }
        Set<String> allowed = AuditCheckTypeEnum.valuesSet();
        return enabledChecks.stream().allMatch(allowed::contains);
    }
}
