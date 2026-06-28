package com.ithsd.smart_tender.pojo.enums;

import java.util.Arrays;
import java.util.Set;
import java.util.stream.Collectors;

public enum AuditCheckTypeEnum {
    BUDGET("budget", AuditStageEnum.CHECK_BUDGET),
    DEMAND("demand", AuditStageEnum.CHECK_DEMAND),
    LEGAL("legal", AuditStageEnum.CHECK_LEGAL);

    private final String value;
    private final AuditStageEnum stage;

    AuditCheckTypeEnum(String value, AuditStageEnum stage) {
        this.value = value;
        this.stage = stage;
    }

    public String getValue() {
        return value;
    }

    public AuditStageEnum getStage() {
        return stage;
    }

    public static Set<String> valuesSet() {
        return Arrays.stream(values()).map(AuditCheckTypeEnum::getValue).collect(Collectors.toSet());
    }

    public static AuditCheckTypeEnum fromValue(String value) {
        return Arrays.stream(values())
                .filter(item -> item.value.equals(value))
                .findFirst()
                .orElseThrow(() -> new IllegalArgumentException("invalid audit check type"));
    }
}
