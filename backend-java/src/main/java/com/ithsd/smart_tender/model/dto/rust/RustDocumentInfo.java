package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

/**
 * Rust {@code GET /api/v1/documents/:id} 的返回体。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustDocumentInfo {
    private String documentId;
    private String filename;
    private int totalPages;
    private int totalChunks;
    private int vectorCount;
}
