package com.ithsd.smart_tender.model.enums;

import java.util.Arrays;

public enum AuditTaskStatusEnum {
    PENDING(0, "pending"),
    PROCESSING(1, "processing"),
    COMPLETED(2, "completed"),
    FAILED(3, "failed");

    private final Integer code;
    private final String value;

    AuditTaskStatusEnum(Integer code, String value) {
        this.code = code;
        this.value = value;
    }

    public Integer getCode() {
        return code;
    }

    public String getValue() {
        return value;
    }

    public static AuditTaskStatusEnum fromCode(Integer code) {
        return Arrays.stream(values())
                .filter(item -> item.code.equals(code))
                .findFirst()
                .orElseThrow(() -> new IllegalArgumentException("invalid task status code"));
    }
}
