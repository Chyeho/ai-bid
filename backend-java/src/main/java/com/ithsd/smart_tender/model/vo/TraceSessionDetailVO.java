package com.ithsd.smart_tender.model.vo;

import java.util.List;

/**
 * 追溯会话详情 VO — 对应 GET /api/traces/{sessionId}。
 *
 * <p>包含 session 摘要 + 全部 trace events（按 turn 排序）。</p>
 */
public class TraceSessionDetailVO {
    private TraceSessionVO session;
    private List<TraceEventVO> events;

    public TraceSessionDetailVO() {}

    public TraceSessionVO getSession() { return session; }
    public void setSession(TraceSessionVO session) { this.session = session; }

    public List<TraceEventVO> getEvents() { return events; }
    public void setEvents(List<TraceEventVO> events) { this.events = events; }
}
