package com.ithsd.smart_tender.sse;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "audit.sse")
public class AuditSseProperties {
    private int replayMaxEvents = 100;

    public int getReplayMaxEvents() {
        return replayMaxEvents;
    }

    public void setReplayMaxEvents(int replayMaxEvents) {
        this.replayMaxEvents = replayMaxEvents;
    }
}
