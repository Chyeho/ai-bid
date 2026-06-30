package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonInclude;
import lombok.Data;

import java.util.ArrayList;
import java.util.List;

/**
 * Rust {@code POST /api/v1/documents/:id/review} 请求体。
 */
@Data
@JsonInclude(JsonInclude.Include.NON_NULL)
public class RustReviewRequest {
    private List<String> chunkIds = new ArrayList<>();
    private Integer maxClauses;
    private List<String> enabledAgents;
}
