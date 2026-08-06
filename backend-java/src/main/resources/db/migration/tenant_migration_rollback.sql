-- Manual-only rollback for a disposable pre-backfill environment.
-- Normal incident rollback is app-level: disable tenant.enforce and any tenant
-- gray rollout, pause tenant-sensitive async/Rust consumers, and retain the
-- Expand columns/tables and any tenant data. Do not run this through Flyway.
--
-- The default NO guard is intentional. Set the variable to YES only after
-- confirming that dual-write/enforce are off, no tenant rows were created, all
-- resource tenant_id values are NULL, and a restorable schema backup exists.

SET @tenant_expand_rollback_confirmed = COALESCE(@tenant_expand_rollback_confirmed, 'NO');

DELIMITER $$

DROP PROCEDURE IF EXISTS `tenant_migration_rollback_expand`$$
CREATE PROCEDURE `tenant_migration_rollback_expand`()
BEGIN
  IF COALESCE(@tenant_expand_rollback_confirmed, 'NO') <> 'YES' THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'Rollback is disabled. Set @tenant_expand_rollback_confirmed = YES after the pre-backfill checks.';
  END IF;

  IF EXISTS (SELECT 1 FROM `tenant` LIMIT 1)
     OR EXISTS (SELECT 1 FROM `tenant_member` LIMIT 1)
     OR EXISTS (SELECT 1 FROM `tenant_invitation` LIMIT 1)
     OR EXISTS (SELECT 1 FROM `tenant_audit_log` LIMIT 1) THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'Tenant data exists. Use feature-flag rollback and retain the Expand schema.';
  END IF;

  IF EXISTS (SELECT 1 FROM `project` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `bid_document` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `audit_task` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `audit_issue` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `audit_report` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `audit_task_event` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `knowledge_file` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `knowledge_chunk` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `chat_message` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `document_parse_job` WHERE `tenant_id` IS NOT NULL LIMIT 1)
     OR EXISTS (SELECT 1 FROM `rag_trigger_outbox` WHERE `tenant_id` IS NOT NULL LIMIT 1) THEN
    SIGNAL SQLSTATE '45000'
      SET MESSAGE_TEXT = 'A resource has a tenant_id. Rollback is unsafe after backfill.';
  END IF;

  ALTER TABLE `rag_trigger_outbox`
    DROP INDEX `idx_rag_trigger_outbox_tenant_id_file_id_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `document_parse_job`
    DROP INDEX `idx_document_parse_job_tenant_id_file_id_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `chat_message`
    DROP INDEX `idx_chat_message_tenant_id_project_id_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `knowledge_chunk`
    DROP INDEX `idx_knowledge_chunk_tenant_id_file_id_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `knowledge_file`
    DROP INDEX `idx_knowledge_file_tenant_id_upload_time_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `audit_task_event`
    DROP INDEX `idx_audit_task_event_tenant_id_task_id_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `audit_report`
    DROP INDEX `idx_audit_report_tenant_id_audit_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `audit_issue`
    DROP INDEX `idx_audit_issue_tenant_id_audit_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `audit_task`
    DROP INDEX `idx_audit_task_tenant_id_bid_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `bid_document`
    DROP INDEX `idx_bid_document_tenant_id_project_id`,
    DROP COLUMN `tenant_id`;
  ALTER TABLE `project`
    DROP INDEX `idx_project_tenant_id_user_id`,
    DROP COLUMN `tenant_id`;

  DROP TABLE `tenant_audit_log`;
  DROP TABLE `tenant_invitation`;
  DROP TABLE `tenant_member`;
  DROP TABLE `tenant`;
END$$

CALL `tenant_migration_rollback_expand`()$$
DROP PROCEDURE IF EXISTS `tenant_migration_rollback_expand`$$

DELIMITER ;
