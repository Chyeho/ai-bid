package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

import java.util.ArrayList;
import java.util.List;

/**
 * Rust ChatResponse — ChatAgent 的返回结构。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustChatResponse {
    /** 自然语言回答（含 [b_xxx] 标记，前端渲染链接） */
    private String answer;
    /** 推理链（按 ReAct turn 顺序，每条为 LLM 在该轮的 thought） */
    private List<String> reasoning = new ArrayList<>();
    /** 原文引用 */
    private List<BlockRef> references = new ArrayList<>();
    /** 法规/案例引用 */
    private List<KnowledgeRef> knowledgeRefs = new ArrayList<>();
    /** 置信度（仅合规判断时） */
    private Float confidence;
    /** 建议下一步操作 */
    private List<String> suggestedActions = new ArrayList<>();

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class BlockRef {
        private String blockId;
        private String quote;
        private String snippet;
        private int page;
    }

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class KnowledgeRef {
        private String refType;   // "law" | "case" | "negative_list"
        private String title;
        private String excerpt;
        private String sourceUrl;
    }
}
