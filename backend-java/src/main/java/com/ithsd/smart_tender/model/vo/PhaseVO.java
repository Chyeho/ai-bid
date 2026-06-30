package com.ithsd.smart_tender.model.vo;

/**
 * 管线阶段切换 VO — 对应 Rust ReviewEvent::Phase。
 * 前端用此更新进度条阶段标签。
 */
public class PhaseVO {
    private String phase;       // route / execute / merge / legal_verify / blind_spot / debate / triage
    private int phaseIndex;     // 1-7
    private int totalPhases;    // 7
    private String message;     // "7 个 Agent 并行审查中..."

    public PhaseVO() {}

    public String getPhase() { return phase; }
    public void setPhase(String phase) { this.phase = phase; }

    public int getPhaseIndex() { return phaseIndex; }
    public void setPhaseIndex(int phaseIndex) { this.phaseIndex = phaseIndex; }

    public int getTotalPhases() { return totalPhases; }
    public void setTotalPhases(int totalPhases) { this.totalPhases = totalPhases; }

    public String getMessage() { return message; }
    public void setMessage(String message) { this.message = message; }
}
