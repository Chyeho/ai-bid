package com.ithsd.smart_tender.service.queue;

public interface AuditTaskDispatcher {
    void dispatch(String taskId);
}
