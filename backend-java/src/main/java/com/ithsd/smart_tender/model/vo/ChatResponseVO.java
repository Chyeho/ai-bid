package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;
import java.util.List;

/**
 * Chat 响应 — 对齐 Rust ChatResponse 结构。
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ChatResponseVO implements Serializable {
    private String content;
    private List<ChatCitationVO> citations;
    private Float confidence;
    private List<String> suggestedActions;

    /** 结构化引用 — 对应 Rust BlockRef 或 KnowledgeRef */
    @Data
    @Builder
    @NoArgsConstructor
    @AllArgsConstructor
    public static class ChatCitationVO implements Serializable {
        /** "block" | "law" | "case" | "negative_list" */
        private String type;
        // BlockRef 字段
        private String blockId;
        private String quote;
        private String snippet;
        private Integer page;
        // KnowledgeRef 字段
        private String title;
        private String excerpt;
        private String sourceUrl;
    }
}
