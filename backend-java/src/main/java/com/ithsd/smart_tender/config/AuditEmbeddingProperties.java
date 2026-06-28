package com.ithsd.smart_tender.config;

import lombok.Data;
import org.springframework.boot.context.properties.ConfigurationProperties;
import org.springframework.stereotype.Component;

@Data
@Component
@ConfigurationProperties(prefix = "audit.embedding")
public class AuditEmbeddingProperties {

    private String provider = "local";
    private int localDimension = 1024;
    private Openai openai = new Openai();

    @Data
    public static class Openai {
        private String baseUrl = "https://api.openai.com/v1";
        private String apiKey = "";
        private String model = "text-embedding-3-large";
        private int maxSegmentsPerBatch = 16;
        private long timeoutMs = 60000;
    }
}
