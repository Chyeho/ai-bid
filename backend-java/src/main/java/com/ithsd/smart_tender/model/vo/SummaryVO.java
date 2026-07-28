package com.ithsd.smart_tender.model.vo;

/**
 * 审核问题汇总 — 4 级风险统计（对齐 Rust RiskSeverity）。
 */
public class SummaryVO {
    private Integer totalIssues;
    private Integer critical;
    private Integer high;
    private Integer medium;
    private Integer low;
    private Integer info;

    public Integer getTotalIssues() { return totalIssues; }
    public void setTotalIssues(Integer totalIssues) { this.totalIssues = totalIssues; }
    public Integer getCritical() { return critical; }
    public void setCritical(Integer critical) { this.critical = critical; }
    public Integer getHigh() { return high; }
    public void setHigh(Integer high) { this.high = high; }
    public Integer getMedium() { return medium; }
    public void setMedium(Integer medium) { this.medium = medium; }
    public Integer getLow() { return low; }
    public void setLow(Integer low) { this.low = low; }
    public Integer getInfo() { return info; }
    public void setInfo(Integer info) { this.info = info; }

    // ── Rust RoutingSummary 扩展 ──
    private Integer totalClauses;
    private java.util.Map<String, Integer> agentClauseCounts;
    private Integer legalVerifyCount;
    private Integer blindSpotFindings;

    public Integer getTotalClauses() { return totalClauses; }
    public void setTotalClauses(Integer totalClauses) { this.totalClauses = totalClauses; }
    public java.util.Map<String, Integer> getAgentClauseCounts() { return agentClauseCounts; }
    public void setAgentClauseCounts(java.util.Map<String, Integer> agentClauseCounts) { this.agentClauseCounts = agentClauseCounts; }
    public Integer getLegalVerifyCount() { return legalVerifyCount; }
    public void setLegalVerifyCount(Integer legalVerifyCount) { this.legalVerifyCount = legalVerifyCount; }
    public Integer getBlindSpotFindings() { return blindSpotFindings; }
    public void setBlindSpotFindings(Integer blindSpotFindings) { this.blindSpotFindings = blindSpotFindings; }
}
