ALTER TABLE `audit_issue`
  ADD COLUMN `is_critical` tinyint(1) NOT NULL DEFAULT 0 COMMENT '是否重大/红线问题' AFTER `severity`,
  ADD COLUMN `critical_reason` text COMMENT '重大问题判定依据' AFTER `is_critical`;

CREATE INDEX `idx_audit_issue_critical` ON `audit_issue` (`is_critical`);
