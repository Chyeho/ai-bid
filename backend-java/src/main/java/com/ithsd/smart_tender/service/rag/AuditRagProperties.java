package com.ithsd.smart_tender.service.rag;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "audit.rag")
public class AuditRagProperties {
    private boolean enabled = true;
    private int topK = 2;
    private String provider = "stub";
    private boolean fallbackToStub = true;
    private boolean httpEnabled = false;

    public boolean isEnabled() {
        return enabled;
    }

    public void setEnabled(boolean enabled) {
        this.enabled = enabled;
    }

    public int getTopK() {
        return topK;
    }

    public void setTopK(int topK) {
        this.topK = topK;
    }

    public String getProvider() {
        return provider;
    }

    public void setProvider(String provider) {
        this.provider = provider;
    }

    public boolean isFallbackToStub() {
        return fallbackToStub;
    }

    public void setFallbackToStub(boolean fallbackToStub) {
        this.fallbackToStub = fallbackToStub;
    }

    public boolean isHttpEnabled() {
        return httpEnabled;
    }

    public void setHttpEnabled(boolean httpEnabled) {
        this.httpEnabled = httpEnabled;
    }
}
