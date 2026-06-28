package com.ithsd.smart_tender.service;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.stereotype.Service;
import org.springframework.web.client.RestTemplate;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

@Service
@Slf4j
@RequiredArgsConstructor
public class RagService {

    private final RestTemplate restTemplate = new RestTemplate();
    private final ObjectMapper objectMapper = new ObjectMapper();

    @Value("${rag.base-url}")
    private String ragBaseUrl;

    /**
     * 获取原始错误（上下文）
     */
    public List<Object> getErrors(String documentId) {
        String url = ragBaseUrl + "/errors?document_id=" + documentId;
        try {
            ResponseEntity<JsonNode> response = restTemplate.getForEntity(url, JsonNode.class);
            if (response.getStatusCode().is2xxSuccessful() && response.getBody() != null) {
                JsonNode issuesNode = response.getBody().get("issues");
                List<Object> issues = new ArrayList<>();
                if (issuesNode != null && issuesNode.isArray()) {
                    for (JsonNode issue : issuesNode) {
                        issues.add(objectMapper.convertValue(issue, Object.class));
                    }
                }
                return issues;
            }
        } catch (Exception e) {
            log.error("Failed to get errors from RAG: {}", e.getMessage());
        }
        return new ArrayList<>();
    }

    /**
     * 生成补充回答
     */
    public Map<String, Object> generateSupplement(String documentId, String userMessage, List<Object> knownIssues) {
        return generateSupplement(documentId, userMessage, knownIssues, null, null);
    }

    public Map<String, Object> generateSupplement(String documentId, String userMessage, List<Object> knownIssues, String queryInstruction) {
        return generateSupplement(documentId, userMessage, knownIssues, queryInstruction, null);
    }

    public Map<String, Object> generateSupplement(String documentId, String userMessage, List<Object> knownIssues, String queryInstruction, String mode) {
        return generateSupplement(documentId, null, userMessage, knownIssues, queryInstruction, mode);
    }

    public Map<String, Object> generateSupplement(String documentId, Long bidFileId, String userMessage, List<Object> knownIssues, String queryInstruction) {
        return generateSupplement(documentId, bidFileId, userMessage, knownIssues, queryInstruction, null);
    }

    public Map<String, Object> generateSupplement(String documentId, Long bidFileId, String userMessage, List<Object> knownIssues, String queryInstruction, String mode) {
        return generateSupplement(documentId, bidFileId, userMessage, knownIssues, queryInstruction, mode, null);
    }

    public Map<String, Object> generateSupplement(String documentId, Long bidFileId, String userMessage, List<Object> knownIssues, String queryInstruction, String mode, Integer topK) {
        String url = ragBaseUrl + "/generate-supplement";
        
        Map<String, Object> requestBody = new HashMap<>();
        requestBody.put("document_id", documentId);
        requestBody.put("collection_name", "tender_rag");
        requestBody.put("user_message", userMessage);
        requestBody.put("known_issues", knownIssues);
        if (queryInstruction != null && !queryInstruction.isBlank()) {
            requestBody.put("query_instruction", queryInstruction);
        }
        if (mode != null && !mode.isBlank()) {
            requestBody.put("mode", mode);
        }
        if (topK != null && topK > 0) {
            requestBody.put("top_k", topK);
        }
        if (bidFileId != null) {
            Map<String, Object> filters = new HashMap<>();
            filters.put("file_id", bidFileId);
            filters.put("source_type", "tender");
            if (documentId != null && !documentId.isBlank()) {
                filters.put("document_id", documentId);
            }
            requestBody.put("filters", filters);
            // Bind retrieval to current audited file only.
            requestBody.put("current_file_only", true);
        }
        // 可选参数，这里使用默认值或按需添加
        // requestBody.put("top_k", 5);
        
        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        HttpEntity<Map<String, Object>> entity = new HttpEntity<>(requestBody, headers);

        try {
            ResponseEntity<Map> response = restTemplate.postForEntity(url, entity, Map.class);
            if (response.getStatusCode().is2xxSuccessful()) {
                return response.getBody();
            }
        } catch (Exception e) {
            log.error("Failed to generate supplement from RAG: {}", e.getMessage());
            // 降级为返回 null，让上层用友好提示而不是抛 500
            return null;
        }
        return null;
    }

    /**
     * 增量入库：保存对话到反馈日志（jsonl），可选是否同步写入知识库
     */
    public void ingestFeedback(String documentId, String content, Long userId, boolean alsoIngestKb) {
        String url = ragBaseUrl + "/ingest-feedback";
        
        List<Map<String, Object>> requestList = new ArrayList<>();
        Map<String, Object> item = new HashMap<>();
        item.put("document_id", documentId);
        item.put("contents", content);
        item.put("source", "supplement");
        item.put("created_by", userId == null ? "unknown" : String.valueOf(userId));
        item.put("created_at", String.valueOf(System.currentTimeMillis() / 1000)); // 秒级时间戳，字符串
        
        // meta 信息，可选
        Map<String, Object> meta = new HashMap<>();
        meta.put("type", "chat_history");
        item.put("meta", meta);
        
        requestList.add(item);

        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        HttpEntity<List<Map<String, Object>>> entity = new HttpEntity<>(requestList, headers);

        try {
            restTemplate.postForEntity(url, entity, Void.class);
            log.info("Successfully ingested feedback to RAG for document: {}", documentId);
            
            if (alsoIngestKb) {
                ingestChunkToKb(documentId, content, userId);
            }
            
        } catch (Exception e) {
            log.error("Failed to ingest feedback to RAG: {}", e.getMessage());
            // 入库失败不阻断主流程，仅记录日志
        }
    }

    /**
     * 将文本作为一条规范知识入库到 knowledge_base 并增量重建索引
     */
    public void ingestChunkToKb(String documentId, String content, Long userId) {
        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        String ingestChunksUrl = ragBaseUrl + "/ingest-chunks";
        Map<String, Object> chunk = new HashMap<>();
        chunk.put("id", "kb-" + System.currentTimeMillis());
        chunk.put("contents", content);
        Map<String, Object> chunkMeta = new HashMap<>();
        chunkMeta.put("document_id", "kb_norm_" + (documentId == null ? "unknown" : documentId));
        chunkMeta.put("title_path", "对话归纳/" + (documentId == null ? "unknown" : documentId));
        chunkMeta.put("section_name", "对话反馈");
        chunkMeta.put("source", "chat_supplement");
        chunkMeta.put("created_by", userId == null ? "unknown" : String.valueOf(userId));
        chunkMeta.put("created_at", String.valueOf(System.currentTimeMillis() / 1000));
        chunk.put("meta", chunkMeta);
        Map<String, Object> ingestChunksBody = new HashMap<>();
        List<Map<String, Object>> chunks = new ArrayList<>();
        chunks.add(chunk);
        ingestChunksBody.put("chunks", chunks);
        ingestChunksBody.put("append", true);
        ingestChunksBody.put("namespace", "kb");
        HttpEntity<Map<String, Object>> ingestChunksEntity = new HttpEntity<>(ingestChunksBody, headers);
        try {
            ResponseEntity<Map> r = restTemplate.postForEntity(ingestChunksUrl, ingestChunksEntity, Map.class);
            if (!r.getStatusCode().is2xxSuccessful()) {
                log.warn("ingest-chunks (kb) returned non-2xx: {}", r.getStatusCode());
            } else {
                Object count = r.getBody() != null ? r.getBody().get("count") : null;
                Object corpus = r.getBody() != null ? r.getBody().get("corpus_path") : null;
                log.info("ingest-chunks (kb) ok: count={}, corpus={}", count, corpus);
            }
        } catch (Exception e) {
            log.warn("ingest-chunks (kb) failed: {}", e.getMessage());
        }

        String buildIndexUrl = ragBaseUrl + "/build-index";
        Map<String, Object> buildBody = new HashMap<>();
        buildBody.put("collection_name", "knowledge_base");
        buildBody.put("overwrite", false);
        buildBody.put("namespace", "kb");
        HttpEntity<Map<String, Object>> buildEntity = new HttpEntity<>(buildBody, headers);
        try {
            ResponseEntity<Map> r2 = restTemplate.postForEntity(buildIndexUrl, buildEntity, Map.class);
            if (!r2.getStatusCode().is2xxSuccessful()) {
                log.warn("build-index (knowledge_base) returned non-2xx: {}", r2.getStatusCode());
            } else {
                log.info("build-index ok: collection=knowledge_base, overwrite=false, namespace=kb");
            }
        } catch (Exception e) {
            log.warn("build-index (knowledge_base) failed: {}", e.getMessage());
        }
    }

    /**
     * 将文本作为补充错误知识入库到 audit_feedback 并增量重建索引
     */
    public void ingestChunkToFeedback(String documentId, String content, Long userId) {
        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        String ingestChunksUrl = ragBaseUrl + "/ingest-chunks";
        Map<String, Object> chunk = new HashMap<>();
        chunk.put("id", "fb-" + System.currentTimeMillis());
        chunk.put("contents", content);
        Map<String, Object> chunkMeta = new HashMap<>();
        chunkMeta.put("document_id", "fb_norm_" + (documentId == null ? "unknown" : documentId));
        chunkMeta.put("title_path", "补充错误归纳/" + (documentId == null ? "unknown" : documentId));
        chunkMeta.put("section_name", "补充错误反馈");
        chunkMeta.put("source", "chat_supplement");
        chunkMeta.put("source_type", "feedback");
        chunkMeta.put("created_by", userId == null ? "unknown" : String.valueOf(userId));
        chunkMeta.put("created_at", String.valueOf(System.currentTimeMillis() / 1000));
        chunk.put("meta", chunkMeta);
        Map<String, Object> ingestChunksBody = new HashMap<>();
        List<Map<String, Object>> chunks = new ArrayList<>();
        chunks.add(chunk);
        ingestChunksBody.put("chunks", chunks);
        ingestChunksBody.put("append", true);
        ingestChunksBody.put("namespace", "feedback");
        HttpEntity<Map<String, Object>> ingestChunksEntity = new HttpEntity<>(ingestChunksBody, headers);
        boolean ingestOk = false;
        try {
            ResponseEntity<Map> r = restTemplate.postForEntity(ingestChunksUrl, ingestChunksEntity, Map.class);
            if (!r.getStatusCode().is2xxSuccessful()) {
                log.warn("ingest-chunks (feedback) returned non-2xx: {}", r.getStatusCode());
            } else {
                Object count = r.getBody() != null ? r.getBody().get("count") : null;
                Object corpus = r.getBody() != null ? r.getBody().get("corpus_path") : null;
                log.info("ingest-chunks (feedback) ok: count={}, corpus={}", count, corpus);
                ingestOk = true;
            }
        } catch (Exception e) {
            log.warn("ingest-chunks (feedback) failed: {}", e.getMessage());
        }

        String buildIndexUrl = ragBaseUrl + "/build-index";
        Map<String, Object> buildBody = new HashMap<>();
        buildBody.put("collection_name", "audit_feedback");
        buildBody.put("overwrite", false);
        buildBody.put("namespace", "feedback");
        HttpEntity<Map<String, Object>> buildEntity = new HttpEntity<>(buildBody, headers);
        try {
            ResponseEntity<Map> r2 = restTemplate.postForEntity(buildIndexUrl, buildEntity, Map.class);
            if (!r2.getStatusCode().is2xxSuccessful()) {
                log.warn("build-index (audit_feedback) returned non-2xx: {}", r2.getStatusCode());
            } else {
                log.info("build-index ok: collection=audit_feedback, overwrite=false, namespace=feedback");
            }
        } catch (Exception e) {
            log.warn("build-index (audit_feedback) failed: {}", e.getMessage());
        }
        if (ingestOk) {
            invalidateAuditCache(documentId, false);
        }
    }

    public void invalidateAuditCache(String documentId, boolean clearAll) {
        String url = ragBaseUrl + "/rag-audit/cache/invalidate";
        Map<String, Object> body = new HashMap<>();
        body.put("document_id", documentId);
        body.put("clear_all", clearAll);
        HttpHeaders headers = new HttpHeaders();
        headers.setContentType(MediaType.APPLICATION_JSON);
        HttpEntity<Map<String, Object>> entity = new HttpEntity<>(body, headers);
        try {
            ResponseEntity<Map> response = restTemplate.postForEntity(url, entity, Map.class);
            if (response.getStatusCode().is2xxSuccessful()) {
                log.info("invalidate-audit-cache ok: documentId={}", documentId);
            } else {
                log.warn("invalidate-audit-cache non-2xx: documentId={}, status={}", documentId, response.getStatusCode());
            }
        } catch (Exception e) {
            log.warn("invalidate-audit-cache failed: documentId={}, err={}", documentId, e.getMessage());
        }
    }

    /**
     * 归纳对话：将用户输入与AI回答压缩为规则化条目文本
     */
    public String normalizeConversation(String documentId, String userMessage, String aiContent) {
        List<Object> known = new ArrayList<>();
        String dialog = "Q: " + (userMessage == null ? "" : userMessage) + "\n"
                + "A: " + (aiContent == null ? "" : aiContent);
        Map<String, Object> res = generateSupplement(documentId, dialog, known, "normalize");
        if (res == null) {
            return null;
        }
        try {
            List<Map<String, Object>> issues = (List<Map<String, Object>>) res.get("issues");
            if (issues == null || issues.isEmpty()) {
                return null;
            }
            StringBuilder sb = new StringBuilder();
            for (Map<String, Object> it : issues) {
                String title = String.valueOf(it.getOrDefault("title", "归纳条目"));
                String rationale = String.valueOf(it.getOrDefault("rationale", ""));
                sb.append("【").append(title).append("】").append("\n");
                if (rationale != null && !rationale.isBlank()) {
                    sb.append("- 要点：").append(rationale).append("\n");
                }
                sb.append("\n");
            }
            return sb.toString();
        } catch (Exception e) {
            log.warn("normalize conversation failed: {}", e.getMessage());
            return null;
        }
    }
}
