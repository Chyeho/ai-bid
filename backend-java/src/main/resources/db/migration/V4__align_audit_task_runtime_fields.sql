-- 对齐 AuditTask 实体及异步审核流程所需的运行时字段。
ALTER TABLE `audit_task`
    ADD COLUMN `stage` varchar(64) DEFAULT NULL COMMENT '当前审核阶段' AFTER `create_time`,
    ADD COLUMN `progress` int(11) NOT NULL DEFAULT 0 COMMENT '审核进度（0-100）' AFTER `stage`,
    ADD COLUMN `enabled_checks` json DEFAULT NULL COMMENT '启用的审核项' AFTER `progress`,
    ADD COLUMN `failed_stages` json DEFAULT NULL COMMENT '失败的审核阶段' AFTER `enabled_checks`,
    ADD COLUMN `error_msg` varchar(1000) DEFAULT NULL COMMENT '失败原因' AFTER `failed_stages`,
    ADD COLUMN `updated_at` datetime DEFAULT NULL COMMENT '更新时间' AFTER `error_msg`,
    ADD COLUMN `version` bigint(20) NOT NULL DEFAULT 0 COMMENT '乐观锁版本号' AFTER `updated_at`;
