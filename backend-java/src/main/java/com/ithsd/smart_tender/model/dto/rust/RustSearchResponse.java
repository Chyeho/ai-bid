package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

import java.util.ArrayList;
import java.util.List;

/**
 * Rust {@code POST /api/v1/documents/:id/search} 返回体。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustSearchResponse {
    private List<SearchResultGroup> results = new ArrayList<>();

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class SearchResultGroup {
        private String query;
        private List<SearchHitDto> hits = new ArrayList<>();
    }

    @Data
    @JsonIgnoreProperties(ignoreUnknown = true)
    public static class SearchHitDto {
        private String chunkId;
        private String title;
        private float score;
        private String snippet;
        private int pageStart;
    }

    /** 展平所有 groups 的 hits 到一个列表 */
    public List<SearchHitDto> allHits() {
        List<SearchHitDto> all = new ArrayList<>();
        if (results != null) {
            for (SearchResultGroup g : results) {
                if (g.hits != null) {
                    all.addAll(g.hits);
                }
            }
        }
        return all;
    }
}
