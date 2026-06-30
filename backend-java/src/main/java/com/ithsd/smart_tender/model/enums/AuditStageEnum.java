package com.ithsd.smart_tender.model.enums;

/**
 * 审核阶段 — 对齐 Rust 处理管线。
 * <p>Rust 同步执行上传→嵌入→审核，Java 侧仅追踪关键里程碑。</p>
 */
public enum AuditStageEnum {
    /** 文件上传到 Rust + PDF 解析/分块/嵌入中 */
    UPLOADING,
    /** Multi-Agent 审核执行中 */
    REVIEWING,
    /** 结果已持久化，审核完成 */
    SUMMARY
}
