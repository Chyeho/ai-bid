package com.ithsd.smart_tender.model.vo;

import java.util.List;
import java.util.Map;

/**
 * Agent 审查进度 VO — 对应 Rust ReviewEvent::AgentProgress。
 * 前端用此渲染 Multi-Agent 并行审查卡片。
 */
public class AgentProgressVO {
    private String agentId;
    private String agentLabel;
    private int clausesDone;
    private int clausesTotal;
    private int rawFindings;
    private String status; // running / completed / failed

    public AgentProgressVO() {}

    public String getAgentId() { return agentId; }
    public void setAgentId(String agentId) { this.agentId = agentId; }

    public String getAgentLabel() { return agentLabel; }
    public void setAgentLabel(String agentLabel) { this.agentLabel = agentLabel; }

    public int getClausesDone() { return clausesDone; }
    public void setClausesDone(int clausesDone) { this.clausesDone = clausesDone; }

    public int getClausesTotal() { return clausesTotal; }
    public void setClausesTotal(int clausesTotal) { this.clausesTotal = clausesTotal; }

    public int getRawFindings() { return rawFindings; }
    public void setRawFindings(int rawFindings) { this.rawFindings = rawFindings; }

    public String getStatus() { return status; }
    public void setStatus(String status) { this.status = status; }
}
