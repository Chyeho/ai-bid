package com.ithsd.smart_tender.pojo.enums;

public enum SseEventTypeEnum {
    PROGRESS("progress"),
    ISSUE("issue"),
    COMPLETE("complete");

    private final String eventName;

    SseEventTypeEnum(String eventName) {
        this.eventName = eventName;
    }

    public String getEventName() {
        return eventName;
    }

    public static SseEventTypeEnum fromEventName(String eventName) {
        if (eventName == null) {
            return null;
        }
        for (SseEventTypeEnum value : values()) {
            if (value.eventName.equals(eventName)) {
                return value;
            }
        }
        return null;
    }
}
