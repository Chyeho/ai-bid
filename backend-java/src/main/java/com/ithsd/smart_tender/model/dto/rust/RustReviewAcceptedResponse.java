package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

/**
 * Rust {@code POST /api/v1/documents/:id/review} 202 Accepted 返回体。
 *
 * <p>异步审核模式：POST /review 立即返回 202，后台 Tokio task 执行审核管线。</p>
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustReviewAcceptedResponse {
    /** "accepted" | "conflict" */
    private String status;
    private String documentId;
    private String message;

    public boolean isAccepted() {
        return "accepted".equals(status);
    }

    public boolean isConflict() {
        return "conflict".equals(status);
    }
}
