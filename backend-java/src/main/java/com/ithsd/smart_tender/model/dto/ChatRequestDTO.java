package com.ithsd.smart_tender.model.dto;

import lombok.Data;

import java.io.Serializable;
import java.util.List;

/**
 * Chat 请求 — 对齐 Rust ChatRequest 结构。
 */
@Data
public class ChatRequestDTO implements Serializable {
    private Long projectId;
    private Long bidId;
    private String content;
    private String mode;
    private Boolean saveToKnowledgeBase;
    private Boolean normalizeBeforeSave;
    /** PDF 文本选区（对齐 Rust TextSelection） */
    private TextSelectionDTO selection;

    @Data
    public static class TextSelectionDTO implements Serializable {
        private String text;
        private List<String> blockIds;
        private int page;
        private BBoxDTO bbox;
    }

    @Data
    public static class BBoxDTO implements Serializable {
        private double x0;
        private double top;
        private double x1;
        private double bottom;
    }
}
