package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.vo.TraceEventVO;
import com.ithsd.smart_tender.model.vo.TraceSessionDetailVO;
import com.ithsd.smart_tender.model.vo.TraceSessionVO;

/**
 * 审查追溯服务 — 负责 trace_sessions / trace_events 的持久化与查询。
 *
 * <p>设计文档 §10.1.4-10.1.5.</p>
 */
public interface TraceService {

    /**
     * 摄入一条 SSE trace 事件：
     * 1. 查找或创建 session (taskId, agentName, clauseId)
     * 2. 插入 trace_event
     * 3. 如果是 output_finding 则更新 session 结果字段
     */
    void ingestTraceEvent(String taskId, String docId, TraceEventVO event);

    /** 将指定 task 下所有 status='running' 的 session 标记为 completed。 */
    void markSessionsCompleted(String taskId);

    /** 分页查询某个审核任务下的所有 trace sessions。 */
    PageResult listByTaskId(String taskId, String agent, String severity, int page, int size);

    /** 获取单个 session 的完整详情（session + 所有 events 按 turn 排序）。 */
    TraceSessionDetailVO getSessionDetail(String sessionId);
}
