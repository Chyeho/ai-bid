package com.ithsd.smart_tender.service;

public interface AuditEngineService {
    void start(String taskId);

    /**
     * 从 Rust 已完成的结果恢复 Java 侧孤儿任务，不重新执行模型审核。
     *
     * @return 已完成或成功恢复时返回 true；Rust 结果尚未完成时返回 false
     */
    boolean recover(String taskId);
}
