-- Rust 审核引擎上传文档后返回 UUID，Java 侧需要持久化该关联 ID。
ALTER TABLE `bid_document`
    ADD COLUMN `rust_document_id` varchar(64) DEFAULT NULL COMMENT 'Rust 审核引擎文档ID' AFTER `project_id`,
    ADD INDEX `idx_rust_document_id` (`rust_document_id`);
