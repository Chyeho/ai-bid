package com.ithsd.smart_tender.model.enums;

public enum SseEventTypeEnum {
    PROGRESS("progress"),
    ISSUE("issue"),
    COMPLETE("complete"),
    /** Agent 审查进度（per-agent 卡片） */
    AGENT_PROGRESS("agent_progress"),
    /** ReAct 实时动态（思考/工具调用/结果） */
    TRACE("trace"),
    /** 管线阶段切换 */
    PHASE("phase"),
    /** 阶段性统计快照 */
    STATS("stats"),
    /** 稳定后的风险发现（L1 主视图实时增量） */
    FINDING_ADDED("finding_added"),
    /** Finding 字段变更（降级/辩论裁决） */
    FINDING_UPDATED("finding_updated"),
    /** Finding 被消除（去重合并） */
    FINDING_REMOVED("finding_removed");

    private final String eventName;

    SseEventTypeEnum(String eventName) {
        this.eventName = eventName;
    }

    public String getEventName() {
        return eventName;
    }

    public static SseEventTypeEnum fromEventName(String eventName) {
        if (eventName == null) {
            return null;
        }
        for (SseEventTypeEnum value : values()) {
            if (value.eventName.equals(eventName)) {
                return value;
            }
        }
        return null;
    }
}
