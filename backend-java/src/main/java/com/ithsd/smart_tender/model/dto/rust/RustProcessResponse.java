package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

import java.util.Map;

/**
 * Rust {@code POST /api/v1/documents} 的返回体。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustProcessResponse {
    private String documentId;
    private String filename;
    private int totalPages;
    private int totalBlocks;
    private int totalSections;
    private int totalChunks;
    private double avgChunkSize;
    private int vectorCount;
    private int vectorDimension;
    private String desensitizationMode;
    private int desensitizedItems;
    private Map<String, Integer> desensitizationCounts;
}
