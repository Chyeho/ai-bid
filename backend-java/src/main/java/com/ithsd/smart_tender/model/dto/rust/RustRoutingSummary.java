package com.ithsd.smart_tender.model.dto.rust;

import com.fasterxml.jackson.annotation.JsonIgnoreProperties;
import lombok.Data;

import java.util.Map;

/**
 * Rust RoutingSummary — Coordinator 路由统计。
 */
@Data
@JsonIgnoreProperties(ignoreUnknown = true)
public class RustRoutingSummary {
    private int totalClauses;
    private Map<String, Integer> agentClauseCounts;
    private int highRiskCount;
    private int legalVerifyCount;
    private int blindSpotFindings;
}
