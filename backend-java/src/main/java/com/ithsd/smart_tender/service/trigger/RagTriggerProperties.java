package com.ithsd.smart_tender.service.trigger;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "rag.trigger")
public class RagTriggerProperties {
    private boolean enabled = false;
    private String endpoint = "";
    private Integer timeoutMs = 3000;
    private Integer maxRetry = 3;
    private Integer initialBackoffMs = 1000;
    private Integer pollDelayMs = 2000;

    public boolean isEnabled() {
        return enabled;
    }

    public void setEnabled(boolean enabled) {
        this.enabled = enabled;
    }

    public String getEndpoint() {
        return endpoint;
    }

    public void setEndpoint(String endpoint) {
        this.endpoint = endpoint;
    }

    public Integer getTimeoutMs() {
        return timeoutMs;
    }

    public void setTimeoutMs(Integer timeoutMs) {
        this.timeoutMs = timeoutMs;
    }

    public Integer getMaxRetry() {
        return maxRetry;
    }

    public void setMaxRetry(Integer maxRetry) {
        this.maxRetry = maxRetry;
    }

    public Integer getInitialBackoffMs() {
        return initialBackoffMs;
    }

    public void setInitialBackoffMs(Integer initialBackoffMs) {
        this.initialBackoffMs = initialBackoffMs;
    }

    public Integer getPollDelayMs() {
        return pollDelayMs;
    }

    public void setPollDelayMs(Integer pollDelayMs) {
        this.pollDelayMs = pollDelayMs;
    }
}
