package com.ithsd.smart_tender.service.trigger;

import com.alibaba.fastjson2.JSON;
import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.ithsd.smart_tender.mapper.KnowledgeChunkMapper;
import com.ithsd.smart_tender.mapper.KnowledgeFileMapper;
import com.ithsd.smart_tender.mapper.TenderMapper;
import com.ithsd.smart_tender.pojo.entity.KnowledgeChunk;
import com.ithsd.smart_tender.pojo.entity.KnowledgeFile;
import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;
import com.ithsd.smart_tender.pojo.entity.Tender;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

@Component
public class JdkRagTriggerHttpClient implements RagTriggerHttpClient {
    private static final Logger log = LoggerFactory.getLogger(JdkRagTriggerHttpClient.class);
    private final RagTriggerProperties properties;
    private final HttpClient httpClient;
    private final KnowledgeChunkMapper knowledgeChunkMapper;
    private final KnowledgeFileMapper knowledgeFileMapper;
    private final TenderMapper tenderMapper;

    public JdkRagTriggerHttpClient(
            RagTriggerProperties properties,
            KnowledgeChunkMapper knowledgeChunkMapper,
            KnowledgeFileMapper knowledgeFileMapper,
            TenderMapper tenderMapper
    ) {
        this.properties = properties;
        this.knowledgeChunkMapper = knowledgeChunkMapper;
        this.knowledgeFileMapper = knowledgeFileMapper;
        this.tenderMapper = tenderMapper;
        // Force HTTP/1.1 to avoid protocol-upgrade edge cases with uvicorn/h11 path.
        this.httpClient = HttpClient.newBuilder()
                .version(HttpClient.Version.HTTP_1_1)
                .connectTimeout(Duration.ofMillis(properties.getTimeoutMs()))
                .build();
    }

    @Override
    public TriggerHttpResult postTrigger(RagTriggerOutboxEntity entity) {
        try {
            if (!StringUtils.hasText(entity.getEndpoint())) {
                return new TriggerHttpResult(0, "", false, true, "RAG_TRIGGER_ENDPOINT_EMPTY");
            }

            String baseUrl = entity.getEndpoint().replace("/rag-audit/trigger", ""); // 兼容旧配置
            if (baseUrl.endsWith("/")) {
                baseUrl = baseUrl.substring(0, baseUrl.length() - 1);
            }

            boolean isAuditTask = StringUtils.hasText(entity.getJobId()) && !entity.getJobId().startsWith("file-");

            if (!isAuditTask) {
                // =============== 阶段一：纯入库 (知识库或标书上传触发) ===============
                // 1. 获取文件的所有切片
                LambdaQueryWrapper<KnowledgeChunk> queryWrapper = new LambdaQueryWrapper<>();
                queryWrapper.eq(KnowledgeChunk::getFileId, entity.getFileId());
                List<KnowledgeChunk> chunks = knowledgeChunkMapper.selectList(queryWrapper);

                String namespace = "tender";
                if (entity.getJobId() != null) {
                    if (entity.getJobId().startsWith("file-kb-")) {
                        namespace = "kb";
                    } else if (entity.getJobId().startsWith("file-tender-")) {
                        namespace = "tender";
                    }
                }
                String sourceType = "kb".equals(namespace) ? "knowledge" : "tender";
                String sourceFileName = resolveSourceFileName(entity.getFileId(), sourceType);

                Map<String, Object> ingestPayload = new LinkedHashMap<>();
                List<Map<String, Object>> chunkList = new ArrayList<>();
                int nullContentsCount = 0;
                for (KnowledgeChunk chunk : chunks) {
                    Map<String, Object> chunkData = new LinkedHashMap<>();
                    chunkData.put("id", "file-" + entity.getFileId() + "-chunk-" + chunk.getChunkIndex());
                    String chunkText = chunk.getChunkText();
                    if (chunkText == null) {
                        nullContentsCount++;
                    }
                    chunkData.put("contents", chunkText);
                    
                    Map<String, Object> meta = new LinkedHashMap<>();
                    meta.put("document_id", String.valueOf(entity.getFileId()));
                    meta.put("file_id", entity.getFileId());
                    meta.put("source_type", sourceType);
                    meta.put("chunk_index", chunk.getChunkIndex());
                    meta.put("chunk_id", chunk.getId());
                    if (StringUtils.hasText(sourceFileName)) {
                        meta.put("file_name", sourceFileName);
                    }
                    meta.put("pageNumber", chunk.getPageNumber());
                    meta.put("sectionName", chunk.getSectionName());
                    chunkData.put("meta", meta);
                    
                    chunkList.add(chunkData);
                }

                ingestPayload.put("chunks", chunkList);
                ingestPayload.put("append", true);
                
                ingestPayload.put("namespace", namespace);
                String ingestBody = JSON.toJSONString(ingestPayload);
                boolean ingestBodyEmpty = !StringUtils.hasText(ingestBody);
                String ingestUrl = baseUrl + "/ingest-chunks";
                String requestId = entity.getRequestId();
                String contentType = "application/json";

                // 关键排查日志：确认 Java 端是否真的构造了 body，以及 body 是否为空。
                log.info(
                        "RAG ingest request prepared: requestId={}, jobId={}, fileId={}, namespace={}, url={}, requestHeaders={Content-Type={}, X-Request-Id={}}, chunks={}, nullContents={}, bodyEmpty={}, bodyLen={}, bodyPreview={}",
                        requestId,
                        entity.getJobId(),
                        entity.getFileId(),
                        namespace,
                        ingestUrl,
                        contentType,
                        requestId,
                        chunkList.size(),
                        nullContentsCount,
                        ingestBodyEmpty,
                        ingestBody == null ? 0 : ingestBody.length(),
                        preview(ingestBody)
                );
                if (ingestBodyEmpty) {
                    log.error(
                            "RAG ingest body is empty: requestId={}, jobId={}, fileId={}",
                            entity.getRequestId(),
                            entity.getJobId(),
                            entity.getFileId()
                    );
                }

                // 3. 发送请求到 /ingest-chunks
                HttpRequest ingestRequest = HttpRequest.newBuilder()
                        .uri(URI.create(ingestUrl))
                        .timeout(Duration.ofMillis(properties.getTimeoutMs() + 5000)) // 切片数据可能较大，增加超时
                        .header("Content-Type", contentType)
                        .header("X-Request-Id", requestId == null ? "" : requestId)
                        .POST(HttpRequest.BodyPublishers.ofString(ingestBody == null ? "" : ingestBody))
                        .build();

                HttpResponse<String> ingestResponse = httpClient.send(ingestRequest, HttpResponse.BodyHandlers.ofString());
                int ingestCode = ingestResponse.statusCode();
                log.info(
                        "RAG ingest response: requestId={}, jobId={}, fileId={}, status={}, httpVersion={}, retryable={}, responsePreview={}",
                        entity.getRequestId(),
                        entity.getJobId(),
                        entity.getFileId(),
                        ingestCode,
                        ingestResponse.version(),
                        ingestCode == 408 || ingestCode == 429 || ingestCode >= 500,
                        preview(ingestResponse.body())
                );
                if (ingestCode < 200 || ingestCode >= 300) {
                    boolean retryable = ingestCode == 408 || ingestCode == 429 || ingestCode >= 500;
                    return new TriggerHttpResult(ingestCode, ingestResponse.body(), false, retryable, "INGEST_HTTP_" + ingestCode);
                }

                // 4. 发送请求到 /build-index (通知 Python 端重建索引)
                Map<String, Object> buildIndexPayload = new LinkedHashMap<>();
                buildIndexPayload.put("namespace", namespace);
                buildIndexPayload.put("collection_name", "kb".equals(namespace) ? "knowledge_base" : "tender_rag"); // 根据namespace存入不同集合
                String buildBody = JSON.toJSONString(buildIndexPayload);
                log.info(
                        "RAG build-index request prepared: requestId={}, jobId={}, fileId={}, namespace={}, bodyLen={}, bodyPreview={}",
                        entity.getRequestId(),
                        entity.getJobId(),
                        entity.getFileId(),
                        namespace,
                        buildBody == null ? 0 : buildBody.length(),
                        preview(buildBody)
                );

                HttpRequest buildIndexRequest = HttpRequest.newBuilder()
                        .uri(URI.create(baseUrl + "/build-index"))
                        .timeout(Duration.ofMillis(properties.getTimeoutMs() + 100000)) // build index 耗时非常长，大幅增加超时时间
                        .header("Content-Type", "application/json")
                        .POST(HttpRequest.BodyPublishers.ofString(buildBody == null ? "" : buildBody))
                        .build();

                HttpResponse<String> buildResponse = httpClient.send(buildIndexRequest, HttpResponse.BodyHandlers.ofString());
                int buildCode = buildResponse.statusCode();
                log.info(
                        "RAG build-index response: requestId={}, jobId={}, fileId={}, status={}, retryable={}, responsePreview={}",
                        entity.getRequestId(),
                        entity.getJobId(),
                        entity.getFileId(),
                        buildCode,
                        buildCode == 408 || buildCode == 429 || buildCode >= 500,
                        preview(buildResponse.body())
                );
                boolean retryable = buildCode == 408 || buildCode == 429 || buildCode >= 500;
                return new TriggerHttpResult(buildCode, buildResponse.body(), buildCode >= 200 && buildCode < 300, retryable, buildCode >= 200 && buildCode < 300 ? null : "BUILD_HTTP_" + buildCode);

            } else {
                // =============== 阶段二：触发审核 (由创建审核任务触发) ===============
                String strategyVersion = entity.getStrategyVersion() == null ? "" : entity.getStrategyVersion();
                String strategyLower = strategyVersion.toLowerCase();
                boolean useWebSearch = strategyLower.contains("audit-websearch");
                boolean forceRefresh = strategyLower.contains("force-refresh");
                Integer documentVersion = resolveDocumentVersion(entity.getFileId());
                Map<String, Object> auditPayload = new LinkedHashMap<>();
                auditPayload.put("requestId", entity.getRequestId());
                auditPayload.put("jobId", entity.getJobId()); // 这个就是 taskId
                auditPayload.put("fileId", String.valueOf(entity.getFileId())); // bidId
                auditPayload.put("chunkCount", entity.getChunkCount() != null ? entity.getChunkCount() : 0);
                auditPayload.put("strategyVersion", entity.getStrategyVersion());
                auditPayload.put("payloadHash", entity.getPayloadHash());
                auditPayload.put("collectionName", "tender_rag");
                auditPayload.put("useWebSearch", useWebSearch);
                auditPayload.put("forceRefresh", forceRefresh);
                auditPayload.put("documentVersion", documentVersion == null ? 0 : documentVersion);
                String auditBody = JSON.toJSONString(auditPayload);
                log.info(
                        "RAG audit request prepared: requestId={}, jobId={}, fileId={}, bodyLen={}, bodyPreview={}",
                        entity.getRequestId(),
                        entity.getJobId(),
                        entity.getFileId(),
                        auditBody == null ? 0 : auditBody.length(),
                        preview(auditBody)
                );

                HttpRequest auditRequest = HttpRequest.newBuilder()
                        .uri(URI.create(baseUrl + "/rag-audit/trigger"))
                        .timeout(Duration.ofMillis(properties.getTimeoutMs() + 300000))
                        .header("Content-Type", "application/json")
                        .POST(HttpRequest.BodyPublishers.ofString(auditBody == null ? "" : auditBody))
                        .build();

                HttpResponse<String> auditResponse = httpClient.send(auditRequest, HttpResponse.BodyHandlers.ofString());
                int auditCode = auditResponse.statusCode();
                log.info(
                        "RAG audit response: requestId={}, jobId={}, fileId={}, status={}, retryable={}, responsePreview={}",
                        entity.getRequestId(),
                        entity.getJobId(),
                        entity.getFileId(),
                        auditCode,
                        auditCode == 408 || auditCode == 429 || auditCode >= 500,
                        preview(auditResponse.body())
                );
                boolean auditSuccess = auditCode >= 200 && auditCode < 300;
                boolean auditRetryable = auditCode == 408 || auditCode == 429 || auditCode >= 500;
                return new TriggerHttpResult(auditCode, auditResponse.body(), auditSuccess, auditRetryable, auditSuccess ? null : "AUDIT_HTTP_" + auditCode);
            }

        } catch (Exception ex) {
            log.error(
                    "RAG trigger request failed: requestId={}, jobId={}, fileId={}, error={}",
                    entity == null ? null : entity.getRequestId(),
                    entity == null ? null : entity.getJobId(),
                    entity == null ? null : entity.getFileId(),
                    ex.getMessage(),
                    ex
            );
            return new TriggerHttpResult(0, "", false, true, ex.getMessage());
        }
    }

    private static String preview(String text) {
        if (text == null) {
            return "null";
        }
        String normalized = text.replace("\r", "\\r").replace("\n", "\\n");
        int max = 800;
        if (normalized.length() <= max) {
            return normalized;
        }
        return normalized.substring(0, max) + "...<truncated>";
    }

    private String resolveSourceFileName(Long fileId, String sourceType) {
        if (fileId == null) {
            return null;
        }
        if ("knowledge".equalsIgnoreCase(sourceType)) {
            KnowledgeFile file = knowledgeFileMapper.selectById(fileId);
            return file == null ? null : file.getFileName();
        }
        Tender tender = tenderMapper.selectById(fileId);
        return tender == null ? null : tender.getFileName();
    }

    private Integer resolveDocumentVersion(Long fileId) {
        if (fileId == null) {
            return 0;
        }
        Tender tender = tenderMapper.selectById(fileId);
        if (tender == null || tender.getVersion() == null) {
            return 0;
        }
        return tender.getVersion();
    }
}
