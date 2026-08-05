package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.Data;

import java.util.ArrayList;
import java.util.List;

/**
 * Rust {@code POST /api/v1/documents/:id/chat} 请求体。
 */
@Data
@JsonInclude(JsonInclude.Include.NON_NULL)
public class RustChatRequest {
    private String userInput;
    private RustTextSelection selection;
    private List<RustChatMessageDto> history;
    private Integer maxTurns;

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class RustTextSelection {
        private String text;
        private List<String> blockIds;
        private int page;
        /** Rust TextSelection.bbox — PDF 选区包围盒，前端高亮定位用 */
        private RustBBox bbox;
    }

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class RustBBox {
        private double x0;
        private double top;
        private double x1;
        private double bottom;
    }

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class RustChatMessageDto {
        private String role;
        private String content;
        /** Rust ChatMessageDto.tool_call_id — 对齐多轮工具调用消息 */
        private String toolCallId;
    }
}
