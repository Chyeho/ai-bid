package com.ithsd.smart_tender.service.extract;

import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Component
@ConfigurationProperties(prefix = "audit.extract")
public class AuditExtractProperties {
    private String provider = "stub";
    private boolean fallbackToStub = true;
    private boolean placeholderEnabled = false;

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

    public boolean isPlaceholderEnabled() {
        return placeholderEnabled;
    }

    public void setPlaceholderEnabled(boolean placeholderEnabled) {
        this.placeholderEnabled = placeholderEnabled;
    }
}
