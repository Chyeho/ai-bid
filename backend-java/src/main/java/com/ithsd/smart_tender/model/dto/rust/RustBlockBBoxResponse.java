package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

/**
 * 对应 Rust GET /api/v1/documents/:id/blocks 返回的单个 block BBox 坐标。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustBlockBBoxResponse {
    private String blockId;
    /** 所在页码 (0-based) */
    private int page;
    /** BBox 坐标（PDF points） */
    private BBoxDto bbox;
    /** 原始 PDF 页面宽度 (pt)，用于前端 scale 计算 */
    private double pageWidth;

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class BBoxDto {
        private double x0;
        private double top;
        private double x1;
        private double bottom;
    }
}
