package com.ithsd.smart_tender.sse;

import com.ithsd.smart_tender.model.enums.SseEventTypeEnum;

public class ReplaySseEvent {
    private String eventId;
    private SseEventTypeEnum eventType;
    private Object data;

    public String getEventId() {
        return eventId;
    }

    public void setEventId(String eventId) {
        this.eventId = eventId;
    }

    public SseEventTypeEnum getEventType() {
        return eventType;
    }

    public void setEventType(SseEventTypeEnum eventType) {
        this.eventType = eventType;
    }

    public Object getData() {
        return data;
    }

    public void setData(Object data) {
        this.data = data;
    }
}
