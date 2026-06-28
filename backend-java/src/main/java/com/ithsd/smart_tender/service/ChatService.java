package com.ithsd.smart_tender.service;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.ithsd.smart_tender.context.BaseContext;
import com.ithsd.smart_tender.mapper.ChatMessageMapper;
import com.ithsd.smart_tender.mapper.KnowledgeFileMapper;
import com.ithsd.smart_tender.mapper.TenderMapper;
import com.ithsd.smart_tender.pojo.dto.ChatRequestDTO;
import com.ithsd.smart_tender.pojo.entity.ChatMessage;
import com.ithsd.smart_tender.pojo.entity.KnowledgeFile;
import com.ithsd.smart_tender.pojo.entity.Tender;
import com.ithsd.smart_tender.pojo.vo.ChatMessageVO;
import com.ithsd.smart_tender.pojo.vo.ChatResponseVO;
import org.springframework.beans.BeanUtils;
import java.util.stream.Collectors;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.HashMap;
import java.util.Set;

@Service
@Slf4j
@RequiredArgsConstructor
public class ChatService {

    private final ChatMessageMapper chatMessageMapper;
    private final TenderMapper tenderMapper;
    private final KnowledgeFileMapper knowledgeFileMapper;
    private final RagService ragService;
    private final ObjectMapper objectMapper = new ObjectMapper();

    @Transactional
    public ChatResponseVO chat(ChatRequestDTO requestDTO) {
        Long userId = BaseContext.getCurrentId();
        if (userId == null) {
            userId = 0L;
        }
        Long projectId = requestDTO.getProjectId();
        Long bidId = requestDTO.getBidId();
        
        // 1. 保存用户提问到数据库
        ChatMessage userMsg = ChatMessage.builder()
                .projectId(projectId)
                .bidId(bidId)
                .userId(userId)
                .role("user")
                .content(requestDTO.getContent())
                .createTime(LocalDateTime.now())
                .build();
        chatMessageMapper.insert(userMsg);

        // 2. 获取对应的标书文档ID
        // 直接使用传入的 bidId
        Tender tender = tenderMapper.selectById(bidId);
        String documentId = (tender != null) ? tender.getId().toString() : "unknown";

        String userContent = requestDTO.getContent() == null ? "" : requestDTO.getContent().trim();
        String requestMode = requestDTO.getMode() == null ? "" : requestDTO.getMode().trim().toLowerCase();
        boolean supplementMode = "supplement".equals(requestMode);
        String aiContent;
        List<Object> citations = new ArrayList<>();
        boolean bidScoped = supplementMode || isBidRelatedQuestion(userContent) || isWeakBidRelatedQuestion(userContent);

        if (!bidScoped) {
            aiContent = buildGeneralReply(userContent);
        } else {
            boolean supplementQuestion = supplementMode || isSupplementQuestion(userContent);
            List<Object> knownIssues = ragService.getErrors(documentId);
            LocalDateTime startTime = LocalDateTime.now().minusDays(3);
            List<ChatMessage> history = chatMessageMapper.selectList(
                    new com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper<ChatMessage>()
                            .eq(ChatMessage::getProjectId, projectId)
                            .eq(ChatMessage::getBidId, bidId)
                            .eq(ChatMessage::getUserId, userId)
                            .ge(ChatMessage::getCreateTime, startTime)
                            .orderByAsc(ChatMessage::getCreateTime)
            );
            StringBuilder ctx = new StringBuilder();
            int maxCtx = 6;
            int start = Math.max(0, history.size() - maxCtx);
            for (int i = start; i < history.size(); i++) {
                ChatMessage m = history.get(i);
                if (!"user".equalsIgnoreCase(m.getRole())) {
                    continue;
                }
                ctx.append(m.getRole()).append(": ").append(m.getContent()).append("\n");
            }
            String bidScopeContext = buildBidScopeContext(tender, bidId, documentId);
            String currentQuestion = supplementMode
                    ? "【补充错误模式】请将用户补充内容先润色为专业问题，再给出可执行建议。\n" + requestDTO.getContent()
                    : requestDTO.getContent();
            String augmentedContent = ctx.length() > 0
                    ? bidScopeContext + "\n【历史对话】\n" + ctx + "【当前问题】\n" + currentQuestion
                    : bidScopeContext + "\n【当前问题】\n" + currentQuestion;

            boolean summaryQuestion = isSummaryQuestion(userContent);
            String prompt = summaryQuestion
                    ? buildSummaryPrompt(augmentedContent)
                    : augmentedContent;
            String instruction = summaryQuestion ? "summary" : null;
            String ragMode = supplementMode ? "supplement" : "dialog";
            List<Object> retrievalKnownIssues = summaryQuestion ? new ArrayList<>() : knownIssues;
            int ragTopK = summaryQuestion ? 8 : 5;

            Map<String, Object> ragResponse = ragService.generateSupplement(
                    documentId,
                    bidId,
                    prompt,
                    retrievalKnownIssues,
                    instruction,
                    ragMode,
                    ragTopK
            );
            if (ragResponse != null) {
                List<Map<String, Object>> issues = (List<Map<String, Object>>) ragResponse.get("issues");
                if (issues != null && !issues.isEmpty()) {
                    appendIssueCitations(issues, citations);
                }
                String plainAnswer = firstNonBlank(
                        valueToString(ragResponse.get("answer")),
                        valueToString(ragResponse.get("summary")),
                        valueToString(ragResponse.get("content")),
                        valueToString(ragResponse.get("message"))
                );
                if (plainAnswer != null && !plainAnswer.isBlank()) {
                    aiContent = plainAnswer;
                } else {
                    if (issues != null && !issues.isEmpty()) {
                        if (summaryQuestion) {
                            aiContent = buildSummaryFromIssues(issues);
                        } else if (supplementQuestion) {
                            aiContent = buildSupplementIssuesPayload(issues);
                        } else {
                            aiContent = buildConversationalAnswerFromIssues(issues, citations);
                        }
                    } else {
                        aiContent = summaryQuestion
                                ? "目前没检索到可用于总结的条款片段。你可以问得更具体一些，例如“这份标书的采购内容、服务期限和付款方式分别是什么？”"
                                : "当前未发现新的标书风险点。你可以继续给出具体条款（如付款、验收、违约、资质要求）让我深入检查。";
                    }
                }
            } else {
                aiContent = "AI服务暂时不可用，请稍后再试。";
            }
            List<Object> currentBidCitations = keepCurrentBidCitations(citations, bidId, tender);
            if (currentBidCitations.isEmpty() && supplementMode) {
                String tenderName = tender == null ? "" : valueToString(tender.getFileName());
                aiContent = (tenderName == null || tenderName.isBlank())
                        ? "当前未检索到这份审核文件的有效证据片段，暂时不能给出针对性结论。请先确认该文件已完成解析和索引。"
                        : "当前未检索到《" + tenderName + "》的有效证据片段，暂时不能给出针对性结论。请先确认该文件已完成解析和索引。";
            }
            citations = currentBidCitations;
            aiContent = prependBidHeader(aiContent, tender, bidId);
        }

        // 4. 保存AI回复到数据库
        ChatMessage aiMsg = ChatMessage.builder()
                .projectId(projectId)
                .bidId(bidId)
                .userId(userId)
                .role("assistant")
                .content(aiContent)
                .createTime(LocalDateTime.now())
                .build();
        chatMessageMapper.insert(aiMsg);

        // 5. 如果需要，保存到补充记录集合（默认：先归纳再入库；不再保留原始Q+A反馈落盘）
        if (Boolean.TRUE.equals(requestDTO.getSaveToKnowledgeBase())) {
            String feedbackContent = "Q: " + requestDTO.getContent() + "\nA: " + aiContent;
            boolean normalize = requestDTO.getNormalizeBeforeSave() == null ? true : requestDTO.getNormalizeBeforeSave();
            if (normalize) {
                String normalized = ragService.normalizeConversation(documentId, requestDTO.getContent(), aiContent);
                if (normalized != null && !normalized.isBlank()) {
                    ragService.ingestChunkToFeedback(documentId, normalized, userId);
                } else {
                    // 归纳失败则不入库，避免将口语化Q+A直接写入补充记录集合
                }
            } else {
                // 直接将原始对话加入补充记录集合
                ragService.ingestChunkToFeedback(documentId, feedbackContent, userId);
            }
        }

        List<Object> normalizedCitations = keepCurrentBidCitations(citations, bidId, tender);
        return ChatResponseVO.builder()
                .content(aiContent)
                .citations(normalizedCitations)
                .build();
    }

    private boolean isBidRelatedQuestion(String content) {
        if (content == null || content.isBlank()) {
            return false;
        }
        String normalized = content.toLowerCase();
        Set<String> keywords = Set.of(
                "标书", "招标", "投标", "条款", "付款", "验收", "违约", "资质", "评分", "合同", "预算", "合规",
                "需求", "技术参数", "采购", "风险", "supplement", "issue", "compliance", "tender", "bid"
        );
        for (String keyword : keywords) {
            if (normalized.contains(keyword.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private boolean isSummaryQuestion(String content) {
        if (content == null || content.isBlank()) {
            return false;
        }
        String normalized = content.toLowerCase();
        Set<String> keywords = Set.of(
                "讲的是什么", "讲什么", "主要内容", "概述", "总结", "摘要", "说了什么", "这份标书", "整体内容",
                "核心内容", "主要讲", "overview", "summary", "what is this tender about"
        );
        for (String keyword : keywords) {
            if (normalized.contains(keyword.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private boolean isWeakBidRelatedQuestion(String content) {
        if (content == null || content.isBlank()) {
            return false;
        }
        String normalized = content.toLowerCase();
        Set<String> weakSignals = Set.of(
                "合理吗", "有风险吗", "会不会", "靠谱吗", "偏甲方", "行不行", "有没有坑", "漏了", "补充一下",
                "再看下", "这条", "这一条", "这个点", "该怎么改", "怎么修", "risk", "reasonable", "unfair"
        );
        for (String signal : weakSignals) {
            if (normalized.contains(signal.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private boolean isSupplementQuestion(String content) {
        if (content == null || content.isBlank()) {
            return false;
        }
        String normalized = content.toLowerCase();
        Set<String> keywords = Set.of(
                "补充错误", "补充问题", "新增问题", "遗漏问题", "继续补充", "再找问题", "再补充",
                "补充风险", "漏掉", "遗漏", "supplement", "add issue"
        );
        for (String keyword : keywords) {
            if (normalized.contains(keyword.toLowerCase())) {
                return true;
            }
        }
        return false;
    }

    private String buildSummaryPrompt(String augmentedContent) {
        return "请基于检索到的标书内容，给出面向业务人员的简明总结，要求：\n"
                + "1) 先用2-4句话说明这份标书“是做什么的”；\n"
                + "2) 再用要点列出：采购对象/服务范围、服务期限、付款方式、验收方式、主要风险点；\n"
                + "3) 若某项信息不明确，明确写“未在当前证据中找到”；\n"
                + "4) 不要输出JSON，直接输出自然语言。\n\n"
                + augmentedContent;
    }

    private String buildSummaryFromIssues(List<Map<String, Object>> issues) {
        StringBuilder sb = new StringBuilder();
        sb.append("这份标书目前可见的重点在于以下几类条款风险：\n");
        int limit = Math.min(5, issues.size());
        for (int i = 0; i < limit; i++) {
            Map<String, Object> issue = issues.get(i);
            sb.append("- ")
                    .append(valueToString(issue.get("title")) == null ? "条款风险点" : valueToString(issue.get("title")))
                    .append("（")
                    .append(valueToString(issue.get("severity")) == null ? "未知等级" : valueToString(issue.get("severity")))
                    .append("）\n");
        }
        sb.append("如果你需要，我可以继续按“采购范围、服务周期、付款条件、验收标准”四个维度给你做一版结构化摘要。");
        return sb.toString();
    }

    private String buildSupplementIssuesPayload(List<Map<String, Object>> issues) {
        try {
            // Return raw JSON array so frontend can render structured issue cards.
            return objectMapper.writeValueAsString(issues);
        } catch (Exception e) {
            StringBuilder sb = new StringBuilder("已采纳你的补充错误，整理如下：\n");
            int limit = Math.min(3, issues.size());
            for (int i = 0; i < limit; i++) {
                Map<String, Object> issue = issues.get(i);
                String title = firstNonBlank(valueToString(issue.get("title")), "补充错误项");
                String rationale = firstNonBlank(valueToString(issue.get("rationale")), "该问题需进一步核查。");
                sb.append(i + 1).append(". ").append(title).append("\n");
                sb.append("风险说明：").append(rationale).append("\n");
            }
            return sb.toString().trim();
        }
    }

    private String buildConversationalAnswerFromIssues(List<Map<String, Object>> issues, List<Object> citations) {
        Map<String, Object> first = issues.get(0);
        String title = valueToString(first.get("title"));
        String rationale = valueToString(first.get("rationale"));
        Object suggestionsObj = first.get("suggestions");
        List<?> suggestions = suggestionsObj instanceof List<?> ? (List<?>) suggestionsObj : List.of();
        if (first.containsKey("citations")) {
            citations.addAll((List<Object>) first.get("citations"));
        }
        StringBuilder sb = new StringBuilder();
        if (title != null && !title.isBlank()) {
            sb.append("我先帮你概括一下：").append(title).append("。");
        }
        if (rationale != null && !rationale.isBlank()) {
            if (sb.length() > 0) sb.append("\n");
            sb.append(rationale);
        }
        if (!suggestions.isEmpty()) {
            sb.append("\n\n你可以优先这样处理：");
            int limit = Math.min(3, suggestions.size());
            for (int i = 0; i < limit; i++) {
                sb.append("\n").append(i + 1).append(". ").append(String.valueOf(suggestions.get(i)));
            }
        }
        return sb.toString();
    }

    private void appendIssueCitations(List<Map<String, Object>> issues, List<Object> citations) {
        if (issues == null || citations == null) {
            return;
        }
        for (Map<String, Object> issue : issues) {
            Object citationObj = issue.get("citations");
            if (citationObj instanceof List<?> list && !list.isEmpty()) {
                citations.addAll((List<Object>) list);
            }
        }
    }

    private String valueToString(Object value) {
        return value == null ? null : String.valueOf(value);
    }

    private String firstNonBlank(String... values) {
        if (values == null) {
            return null;
        }
        for (String value : values) {
            if (value != null && !value.isBlank()) {
                return value;
            }
        }
        return null;
    }

    private String buildGeneralReply(String content) {
        if (content == null || content.isBlank()) {
            return "你好，我在。你可以直接问我标书条款问题，我会帮你做风险点补充。";
        }
        String normalized = content.trim().toLowerCase();
        if (Set.of("你好", "hi", "hello", "在吗", "嗨", "哈喽").contains(normalized)) {
            return "你好，我在。你可以把想确认的标书条款发给我，我帮你补充风险点和修改建议。";
        }
        return "收到。这个问题不是标书条款类问题。你可以直接发具体条款内容（如付款、验收、违约、资质）让我继续分析。";
    }

    private String buildBidScopeContext(Tender tender, Long bidId, String documentId) {
        String bidFileName = tender == null ? "" : firstNonBlank(valueToString(tender.getFileName()), valueToString(tender.getBidName()));
        String fileIdText = bidId == null ? "unknown" : String.valueOf(bidId);
        String documentIdText = (documentId == null || documentId.isBlank()) ? "unknown" : documentId;
        if (bidFileName == null || bidFileName.isBlank()) {
            return "【文件范围】仅允许基于当前审核文件作答。file_id=" + fileIdText + "，document_id=" + documentIdText
                    + "。若证据不足请明确说明，不要引用其他文件。";
        }
        return "【文件范围】仅允许基于当前审核文件作答。file_id=" + fileIdText + "，document_id=" + documentIdText
                + "，file_name=" + bidFileName + "。若证据不足请明确说明，不要引用其他文件。";
    }

    private String prependBidHeader(String aiContent, Tender tender, Long bidId) {
        String content = aiContent == null ? "" : aiContent;
        String bidFileName = tender == null ? "" : firstNonBlank(valueToString(tender.getFileName()), valueToString(tender.getBidName()));
        String fileIdText = bidId == null ? "unknown" : String.valueOf(bidId);
        String header = (bidFileName == null || bidFileName.isBlank())
                ? "当前审核文件：ID=" + fileIdText
                : "当前审核文件：" + bidFileName + "（ID=" + fileIdText + "）";
        if (content.startsWith(header)) {
            return content;
        }
        if (content.isBlank()) {
            return header;
        }
        return header + "\n" + content;
    }

    private List<Object> enrichCitations(List<Object> citations, Long bidId, Tender currentTender) {
        if (citations == null || citations.isEmpty()) {
            return citations == null ? new ArrayList<>() : citations;
        }
        Map<Long, Tender> tenderCache = new HashMap<>();
        Map<Long, KnowledgeFile> knowledgeCache = new HashMap<>();
        if (currentTender != null && currentTender.getId() != null) {
            tenderCache.put(currentTender.getId(), currentTender);
        }
        List<Object> result = new ArrayList<>(citations.size());
        for (Object citationObj : citations) {
            if (!(citationObj instanceof Map<?, ?> citationMapRaw)) {
                result.add(citationObj);
                continue;
            }
            Map<String, Object> citationMap = new LinkedHashMap<>();
            for (Map.Entry<?, ?> entry : citationMapRaw.entrySet()) {
                citationMap.put(String.valueOf(entry.getKey()), entry.getValue());
            }
            Map<String, Object> meta = toStringKeyMap(citationMap.get("meta"));
            Long fileId = parseLong(meta.get("file_id"));
            if (fileId == null) {
                fileId = parseLong(meta.get("fileId"));
            }
            if (fileId == null) {
                fileId = parseLong(meta.get("document_id"));
            }
            String fileName = valueToString(meta.get("file_name"));
            if (fileName == null || fileName.isBlank()) {
                fileName = valueToString(meta.get("fileName"));
            }
            String sourceType = valueToString(meta.get("source_type"));
            if (sourceType == null || sourceType.isBlank()) {
                sourceType = valueToString(meta.get("sourceType"));
            }
            if ((fileName == null || fileName.isBlank() || sourceType == null || sourceType.isBlank()) && fileId != null) {
                if (bidId != null && bidId.equals(fileId)) {
                    Tender tender = tenderCache.computeIfAbsent(fileId, tenderMapper::selectById);
                    if (tender != null) {
                        if (fileName == null || fileName.isBlank()) {
                            fileName = tender.getFileName();
                        }
                        if (sourceType == null || sourceType.isBlank()) {
                            sourceType = "tender";
                        }
                    }
                }
                if (sourceType == null || sourceType.isBlank() || "knowledge".equalsIgnoreCase(sourceType)) {
                    KnowledgeFile file = knowledgeCache.computeIfAbsent(fileId, knowledgeFileMapper::selectById);
                    if (file != null) {
                        if (fileName == null || fileName.isBlank()) {
                            fileName = file.getFileName();
                        }
                        sourceType = "knowledge";
                    }
                }
                if ((sourceType == null || sourceType.isBlank()) && (fileName == null || fileName.isBlank())) {
                    Tender tender = tenderCache.computeIfAbsent(fileId, tenderMapper::selectById);
                    if (tender != null) {
                        fileName = tender.getFileName();
                        sourceType = "tender";
                    }
                }
            }
            if (fileId != null) {
                meta.put("fileId", fileId);
            }
            if (fileName != null && !fileName.isBlank()) {
                meta.put("fileName", fileName);
            }
            if (sourceType != null && !sourceType.isBlank()) {
                meta.put("sourceType", sourceType);
            }
            citationMap.put("meta", meta);
            result.add(citationMap);
        }
        return result;
    }

    private List<Object> keepCurrentBidCitations(List<Object> citations, Long bidId, Tender currentTender) {
        if (citations == null || citations.isEmpty()) {
            return new ArrayList<>();
        }
        if (bidId == null) {
            return new ArrayList<>();
        }
        Tender tender = currentTender != null ? currentTender : tenderMapper.selectById(bidId);
        String tenderName = tender == null ? null : tender.getFileName();
        List<Object> result = new ArrayList<>();
        List<Map<String, Object>> fallbackCandidates = new ArrayList<>();
        for (Object citationObj : citations) {
            if (!(citationObj instanceof Map<?, ?> citationMapRaw)) {
                continue;
            }
            Map<String, Object> citationMap = new LinkedHashMap<>();
            for (Map.Entry<?, ?> entry : citationMapRaw.entrySet()) {
                citationMap.put(String.valueOf(entry.getKey()), entry.getValue());
            }
            Map<String, Object> meta = toStringKeyMap(citationMap.get("meta"));
            Long fileId = parseLong(meta.get("fileId"));
            if (fileId == null) {
                fileId = parseLong(meta.get("file_id"));
            }
            String sourceType = valueToString(meta.get("sourceType"));
            if (sourceType == null || sourceType.isBlank()) {
                sourceType = valueToString(meta.get("source_type"));
            }
            String sourceTypeNorm = sourceType == null ? "" : sourceType.trim().toLowerCase();
            if ("knowledge".equals(sourceTypeNorm)) {
                continue;
            }
            if (fileId == null) {
                fileId = parseLong(meta.get("document_id"));
            }
            fallbackCandidates.add(citationMap);
            if (fileId == null || !bidId.equals(fileId)) {
                continue;
            }
            meta.put("fileId", bidId);
            if (tenderName != null && !tenderName.isBlank()) {
                meta.put("fileName", tenderName);
            }
            meta.put("sourceType", "tender");
            citationMap.put("meta", meta);
            result.add(citationMap);
        }
        if (!result.isEmpty()) {
            return result;
        }

        // Controlled fallback:
        // if strict id filtering yields nothing, but all tender citations point to exactly one id,
        // accept that single-id group and keep a mapping hint for troubleshooting.
        Set<Long> uniqueCandidateIds = fallbackCandidates.stream()
                .map(item -> toStringKeyMap(item.get("meta")))
                .map(meta -> {
                    Long id = parseLong(meta.get("fileId"));
                    if (id == null) {
                        id = parseLong(meta.get("file_id"));
                    }
                    if (id == null) {
                        id = parseLong(meta.get("document_id"));
                    }
                    return id;
                })
                .filter(id -> id != null)
                .collect(Collectors.toSet());

        if (uniqueCandidateIds.size() == 1) {
            Long mappedId = uniqueCandidateIds.iterator().next();
            log.warn("Citation id mapping mismatch: bidId={}, candidateId={}", bidId, mappedId);
            for (Map<String, Object> item : fallbackCandidates) {
                Map<String, Object> meta = toStringKeyMap(item.get("meta"));
                Long id = parseLong(meta.get("fileId"));
                if (id == null) {
                    id = parseLong(meta.get("file_id"));
                }
                if (id == null) {
                    id = parseLong(meta.get("document_id"));
                }
                if (id == null || !mappedId.equals(id)) {
                    continue;
                }
                meta.put("sourceType", "tender");
                meta.put("mappedFromId", mappedId);
                meta.put("mappedToBidId", bidId);
                if (tenderName != null && !tenderName.isBlank()) {
                    meta.put("fileName", tenderName);
                }
                item.put("meta", meta);
                result.add(item);
            }
            if (!result.isEmpty()) {
                return result;
            }
        }
        return result;
    }

    private Map<String, Object> toStringKeyMap(Object value) {
        Map<String, Object> result = new LinkedHashMap<>();
        if (!(value instanceof Map<?, ?> raw)) {
            return result;
        }
        for (Map.Entry<?, ?> entry : raw.entrySet()) {
            result.put(String.valueOf(entry.getKey()), entry.getValue());
        }
        return result;
    }

    private Long parseLong(Object value) {
        if (value == null) {
            return null;
        }
        if (value instanceof Number num) {
            return num.longValue();
        }
        try {
            String text = String.valueOf(value).trim();
            if (text.isBlank()) {
                return null;
            }
            return Long.parseLong(text);
        } catch (Exception ignored) {
            return null;
        }
    }

    public List<ChatMessageVO> getHistory(Long projectId, Long bidId, Integer days) {
        Long userId = BaseContext.getCurrentId();
        if (userId == null) {
            userId = 0L;
        }
        int queryDays = (days != null && days > 0) ? days : 10;

        LocalDateTime startTime = LocalDateTime.now().minusDays(queryDays);

        List<ChatMessage> messages = chatMessageMapper.selectList(
                new com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper<ChatMessage>()
                        .eq(ChatMessage::getProjectId, projectId)
                        .eq(ChatMessage::getBidId, bidId)
                        .eq(ChatMessage::getUserId, userId)
                        .ge(ChatMessage::getCreateTime, startTime)
                        .orderByAsc(ChatMessage::getCreateTime)
        );

        return messages.stream().map(msg -> {
            ChatMessageVO vo = new ChatMessageVO();
            BeanUtils.copyProperties(msg, vo);
            return vo;
        }).collect(Collectors.toList());
    }

    @Transactional
    public String commitKnowledge(com.ithsd.smart_tender.pojo.dto.ChatCommitRequestDTO req) {
        Long userId = BaseContext.getCurrentId();
        if (userId == null) {
            userId = 0L;
        }
        Long projectId = req.getProjectId();
        Long bidId = req.getBidId();
        Tender tender = tenderMapper.selectById(bidId);
        String documentId = (tender != null) ? tender.getId().toString() : "unknown";
        String userContent = req.getUserContent();
        String aiContent = req.getAiContent();
        if (userContent == null || userContent.isBlank() || aiContent == null || aiContent.isBlank()) {
            return "暂无补充错误可保存。";
        }
        boolean normalize = req.getNormalizeBeforeSave() == null ? true : req.getNormalizeBeforeSave();
        String commitResult;
        if (normalize) {
            String normalized = ragService.normalizeConversation(documentId, userContent == null ? "" : userContent, aiContent == null ? "" : aiContent);
            if (normalized != null && !normalized.isBlank()) {
                ragService.ingestChunkToFeedback(documentId, normalized, userId);
                commitResult = normalized;
            } else {
                commitResult = "归纳失败，未写入补充记录集合。";
            }
        } else {
            String feedbackContent = "Q: " + (userContent == null ? "" : userContent) + "\nA: " + (aiContent == null ? "" : aiContent);
            ragService.ingestChunkToFeedback(documentId, feedbackContent, userId);
            commitResult = feedbackContent;
        }
        if (commitResult != null && !commitResult.isBlank()) {
            String summaryDisplay = "已保存记录，归纳内容如下：\n" + commitResult;
            chatMessageMapper.insert(
                    ChatMessage.builder()
                            .projectId(projectId)
                            .bidId(bidId)
                            .userId(userId)
                            .role("assistant")
                            .content(summaryDisplay)
                            .createTime(LocalDateTime.now())
                            .build()
            );
        }
        return commitResult;
    }

}
