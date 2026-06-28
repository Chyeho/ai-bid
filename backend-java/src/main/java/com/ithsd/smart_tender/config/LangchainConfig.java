package com.ithsd.smart_tender.config;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.langchain4j.data.embedding.Embedding;
import dev.langchain4j.data.segment.TextSegment;
import dev.langchain4j.model.embedding.EmbeddingModel;
import dev.langchain4j.model.output.Response;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;
import org.springframework.util.StringUtils;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

@Configuration
public class LangchainConfig {

    @Autowired
    private AuditEmbeddingProperties embeddingProperties;

    @Autowired
    private ObjectMapper objectMapper;

    @Bean
    public EmbeddingModel embeddingModel() {
        String provider = normalizeProvider(embeddingProperties.getProvider());
        if ("openai".equals(provider)) {
            return openaiEmbeddingModel();
        }
        return localEmbeddingModel();
    }

    private EmbeddingModel localEmbeddingModel() {
        int dimension = Math.max(8, embeddingProperties.getLocalDimension());
        return textSegments -> {
            List<Embedding> embeddings = new ArrayList<>();
            for (TextSegment ignored : textSegments) {
                float[] vector = new float[dimension];
                embeddings.add(Embedding.from(vector));
            }
            return Response.from(embeddings);
        };
    }

    private EmbeddingModel openaiEmbeddingModel() {
        AuditEmbeddingProperties.Openai cfg = embeddingProperties.getOpenai();
        return textSegments -> {
            if (textSegments == null || textSegments.isEmpty()) {
                return Response.from(new ArrayList<>());
            }
            if (!StringUtils.hasText(cfg.getApiKey())) {
                throw new IllegalStateException("audit.embedding.openai.api-key 为空，无法使用 openai 向量化");
            }
            int batchSize = Math.max(1, cfg.getMaxSegmentsPerBatch());
            HttpClient client = HttpClient.newBuilder()
                    .connectTimeout(Duration.ofMillis(Math.max(1000, cfg.getTimeoutMs())))
                    .build();
            List<Embedding> embeddings = new ArrayList<>();
            for (int i = 0; i < textSegments.size(); i += batchSize) {
                int end = Math.min(i + batchSize, textSegments.size());
                List<String> inputs = new ArrayList<>(end - i);
                for (TextSegment segment : textSegments.subList(i, end)) {
                    String text = segment == null ? "" : segment.text();
                    inputs.add(text == null ? "" : text);
                }
                Map<String, Object> body = new HashMap<>();
                body.put("model", cfg.getModel());
                body.put("input", inputs);
                String url = normalizeOpenAiBaseUrl(cfg.getBaseUrl()) + "/embeddings";
                try {
                    String jsonBody = objectMapper.writeValueAsString(body);
                    HttpRequest request = HttpRequest.newBuilder()
                            .uri(URI.create(url))
                            .timeout(Duration.ofMillis(Math.max(1000, cfg.getTimeoutMs())))
                            .header("Authorization", "Bearer " + cfg.getApiKey())
                            .header("Content-Type", "application/json")
                            .POST(HttpRequest.BodyPublishers.ofString(jsonBody, StandardCharsets.UTF_8))
                            .build();
                    HttpResponse<String> response = client.send(request, HttpResponse.BodyHandlers.ofString(StandardCharsets.UTF_8));
                    if (response.statusCode() < 200 || response.statusCode() >= 300) {
                        throw new IllegalStateException("OpenAI embedding 请求失败，status=" + response.statusCode() + ", body=" + response.body());
                    }
                    JsonNode root = objectMapper.readTree(response.body());
                    JsonNode dataNode = root.path("data");
                    if (!dataNode.isArray()) {
                        throw new IllegalStateException("OpenAI embedding 响应缺少 data 数组");
                    }
                    for (JsonNode item : dataNode) {
                        JsonNode vectorNode = item.path("embedding");
                        if (!vectorNode.isArray()) {
                            continue;
                        }
                        float[] vector = new float[vectorNode.size()];
                        for (int j = 0; j < vectorNode.size(); j++) {
                            vector[j] = (float) vectorNode.get(j).asDouble();
                        }
                        embeddings.add(Embedding.from(vector));
                    }
                } catch (Exception e) {
                    throw new IllegalStateException("OpenAI embedding 调用异常: " + e.getMessage(), e);
                }
            }
            if (embeddings.size() != textSegments.size()) {
                throw new IllegalStateException("OpenAI embedding 返回数量与输入数量不一致: input="
                        + textSegments.size() + ", output=" + embeddings.size());
            }
            return Response.from(embeddings);
        };
    }

    private String normalizeProvider(String provider) {
        return provider == null ? "local" : provider.trim().toLowerCase(Locale.ROOT);
    }

    private String normalizeOpenAiBaseUrl(String baseUrl) {
        String value = StringUtils.hasText(baseUrl) ? baseUrl.trim() : "https://api.openai.com/v1";
        if (value.endsWith("/")) {
            value = value.substring(0, value.length() - 1);
        }
        if (!value.endsWith("/v1")) {
            value = value + "/v1";
        }
        return value;
    }
}
