-- Manual validation for V5/V6. This file is intentionally not a Flyway migration.
-- Run it against the same schema after V6 and save the result with the release
-- evidence. Empty result sets are expected for exception queries.

-- 1. Schema shape: every V1 resource table must have a nullable tenant_id.
SELECT
  c.`table_name`,
  c.`column_name`,
  c.`data_type`,
  c.`is_nullable`
FROM information_schema.`columns` AS c
WHERE c.`table_schema` = DATABASE()
  AND c.`column_name` = 'tenant_id'
  AND c.`table_name` IN (
    'project', 'bid_document', 'audit_task', 'audit_issue', 'audit_report',
    'audit_task_event', 'knowledge_file', 'knowledge_chunk', 'chat_message',
    'document_parse_job', 'rag_trigger_outbox'
  )
ORDER BY c.`table_name`;

-- 2. Tenant-leading indexes created by V5.
SELECT
  s.`table_name`,
  s.`index_name`,
  s.`seq_in_index`,
  s.`column_name`
FROM information_schema.`statistics` AS s
WHERE s.`table_schema` = DATABASE()
  AND s.`index_name` IN (
    'idx_project_tenant_id_user_id',
    'idx_bid_document_tenant_id_project_id',
    'idx_audit_task_tenant_id_bid_id',
    'idx_audit_issue_tenant_id_audit_id',
    'idx_audit_report_tenant_id_audit_id',
    'idx_audit_task_event_tenant_id_task_id_id',
    'idx_knowledge_file_tenant_id_upload_time_id',
    'idx_knowledge_chunk_tenant_id_file_id_id',
    'idx_chat_message_tenant_id_project_id_id',
    'idx_document_parse_job_tenant_id_file_id_id',
    'idx_rag_trigger_outbox_tenant_id_file_id_id'
  )
ORDER BY s.`table_name`, s.`index_name`, s.`seq_in_index`;

-- 3. Null tenant_id counts. Only rows called out by the exception query below
-- may remain NULL before Contract; V6 must not silently invent ownership.
SELECT 'project' AS `source_table`, COUNT(*) AS `total_rows`,
       SUM(`tenant_id` IS NULL) AS `null_tenant_id`
  FROM `project`
UNION ALL
SELECT 'bid_document', COUNT(*), SUM(`tenant_id` IS NULL) FROM `bid_document`
UNION ALL
SELECT 'audit_task', COUNT(*), SUM(`tenant_id` IS NULL) FROM `audit_task`
UNION ALL
SELECT 'audit_issue', COUNT(*), SUM(`tenant_id` IS NULL) FROM `audit_issue`
UNION ALL
SELECT 'audit_report', COUNT(*), SUM(`tenant_id` IS NULL) FROM `audit_report`
UNION ALL
SELECT 'audit_task_event', COUNT(*), SUM(`tenant_id` IS NULL) FROM `audit_task_event`
UNION ALL
SELECT 'knowledge_file', COUNT(*), SUM(`tenant_id` IS NULL) FROM `knowledge_file`
UNION ALL
SELECT 'knowledge_chunk', COUNT(*), SUM(`tenant_id` IS NULL) FROM `knowledge_chunk`
UNION ALL
SELECT 'chat_message', COUNT(*), SUM(`tenant_id` IS NULL) FROM `chat_message`
UNION ALL
SELECT 'document_parse_job', COUNT(*), SUM(`tenant_id` IS NULL) FROM `document_parse_job`
UNION ALL
SELECT 'rag_trigger_outbox', COUNT(*), SUM(`tenant_id` IS NULL) FROM `rag_trigger_outbox`;

-- 4. Every legacy user needs an ACTIVE membership in the backfilled personal
-- tenant. sys_user.status remains a separate authentication gate.
SELECT
  u.`id` AS `user_id`,
  u.`status` AS `sys_user_status`,
  t.`id` AS `tenant_id`,
  tm.`status` AS `member_status`,
  tm.`role`
FROM `sys_user` AS u
LEFT JOIN `tenant` AS t
  ON t.`tenant_code` = CONCAT('user-', u.`id`)
LEFT JOIN `tenant_member` AS tm
  ON tm.`tenant_id` = t.`id`
 AND tm.`user_id` = u.`id`
 AND tm.`status` = 'ACTIVE'
WHERE tm.`id` IS NULL
ORDER BY u.`id`;

-- Owner invariant for non-DELETED tenants.
SELECT
  t.`id` AS `tenant_id`,
  t.`tenant_code`,
  t.`status` AS `tenant_status`,
  t.`owner_user_id`
FROM `tenant` AS t
LEFT JOIN `tenant_member` AS tm
  ON tm.`tenant_id` = t.`id`
 AND tm.`user_id` = t.`owner_user_id`
 AND tm.`role` = 'OWNER'
 AND tm.`status` = 'ACTIVE'
WHERE t.`status` <> 'DELETED'
  AND tm.`id` IS NULL
ORDER BY t.`id`;

-- 5. Current tenant-visible resource counts. These are the counts consumed by
-- tenant-scoped reads; compare them with the saved pre-migration evidence.
SELECT t.`id` AS `tenant_id`, t.`tenant_code`, 'project' AS `resource_type`,
       COUNT(p.`id`) AS `visible_count`
FROM `tenant` AS t LEFT JOIN `project` AS p ON p.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'bid_document', COUNT(b.`id`)
FROM `tenant` AS t LEFT JOIN `bid_document` AS b ON b.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'audit_task', COUNT(a.`id`)
FROM `tenant` AS t LEFT JOIN `audit_task` AS a ON a.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'audit_issue', COUNT(i.`id`)
FROM `tenant` AS t LEFT JOIN `audit_issue` AS i ON i.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'audit_report', COUNT(r.`id`)
FROM `tenant` AS t LEFT JOIN `audit_report` AS r ON r.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'audit_task_event', COUNT(e.`id`)
FROM `tenant` AS t LEFT JOIN `audit_task_event` AS e ON e.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'knowledge_file', COUNT(k.`id`)
FROM `tenant` AS t LEFT JOIN `knowledge_file` AS k ON k.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'knowledge_chunk', COUNT(c.`id`)
FROM `tenant` AS t LEFT JOIN `knowledge_chunk` AS c ON c.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'chat_message', COUNT(m.`id`)
FROM `tenant` AS t LEFT JOIN `chat_message` AS m ON m.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'document_parse_job', COUNT(j.`id`)
FROM `tenant` AS t LEFT JOIN `document_parse_job` AS j ON j.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
UNION ALL
SELECT t.`id`, t.`tenant_code`, 'rag_trigger_outbox', COUNT(o.`id`)
FROM `tenant` AS t LEFT JOIN `rag_trigger_outbox` AS o ON o.`tenant_id` = t.`id`
GROUP BY t.`id`, t.`tenant_code`
ORDER BY `tenant_id`, `resource_type`;

-- Direct legacy-owner versus personal-tenant counts. The comparison is limited
-- to rows with a direct V1 owner and is a best-effort parity check.
SELECT
  u.`id` AS `user_id`,
  (SELECT COUNT(*) FROM `project` AS p WHERE p.`user_id` = u.`id`) AS `legacy_project_count`,
  (SELECT COUNT(*)
     FROM `project` AS p
     JOIN `tenant` AS t ON t.`id` = p.`tenant_id`
    WHERE t.`tenant_code` = CONCAT('user-', u.`id`)
      AND p.`user_id` = u.`id`) AS `tenant_project_count`,
  (SELECT COUNT(*) FROM `knowledge_file` AS k WHERE k.`upload_user_id` = u.`id`) AS `legacy_knowledge_file_count`,
  (SELECT COUNT(*)
     FROM `knowledge_file` AS k
     JOIN `tenant` AS t ON t.`id` = k.`tenant_id`
    WHERE t.`tenant_code` = CONCAT('user-', u.`id`)
      AND k.`upload_user_id` = u.`id`) AS `tenant_knowledge_file_count`,
  (SELECT COUNT(*) FROM `chat_message` AS m WHERE m.`user_id` = u.`id`) AS `legacy_chat_message_count`,
  (SELECT COUNT(*)
     FROM `chat_message` AS m
     JOIN `tenant` AS t ON t.`id` = m.`tenant_id`
    WHERE t.`tenant_code` = CONCAT('user-', u.`id`)
      AND m.`user_id` = u.`id`) AS `tenant_chat_message_count`
FROM `sys_user` AS u
ORDER BY u.`id`;

-- 6. Unresolved rows and conflicts. This is the migration isolation queue:
-- each result is identified by (source_table, source_id) and must be fixed or
-- explicitly excluded before Contract.
SELECT 'project' AS `source_table`, CAST(p.`id` AS CHAR) AS `source_id`,
       CAST(p.`user_id` AS CHAR) AS `candidate_owner`,
       CASE
         WHEN p.`user_id` IS NULL THEN 'user_id is NULL'
         WHEN t.`id` IS NULL THEN 'personal tenant for user_id is missing'
         ELSE 'tenant_id remained NULL'
       END AS `reason`
FROM `project` AS p
LEFT JOIN `tenant` AS t
  ON t.`tenant_code` = CONCAT('user-', p.`user_id`)
WHERE p.`tenant_id` IS NULL
UNION ALL
SELECT 'bid_document', CAST(b.`id` AS CHAR),
       CONCAT('upload_user_id=', COALESCE(CAST(b.`upload_user_id` AS CHAR), 'NULL'),
              '; project_id=', COALESCE(CAST(b.`project_id` AS CHAR), 'NULL')),
       CASE
         WHEN upload_t.`id` IS NOT NULL AND p.`tenant_id` IS NOT NULL
              AND upload_t.`id` <> p.`tenant_id` THEN 'upload user and project owners conflict'
         WHEN upload_t.`id` IS NULL AND p.`tenant_id` IS NULL THEN 'uploader and project owner are unresolved'
         ELSE 'tenant_id remained NULL'
       END
FROM `bid_document` AS b
LEFT JOIN `tenant` AS upload_t
  ON upload_t.`tenant_code` = CONCAT('user-', b.`upload_user_id`)
LEFT JOIN `project` AS p
  ON p.`id` = b.`project_id`
WHERE b.`tenant_id` IS NULL
UNION ALL
SELECT 'audit_task', CAST(a.`id` AS CHAR),
       CONCAT('audit_user_id=', COALESCE(CAST(a.`audit_user_id` AS CHAR), 'NULL'),
              '; bid_id=', CAST(a.`bid_id` AS CHAR)),
       CASE
         WHEN audit_t.`id` IS NOT NULL
              AND COALESCE(b.`tenant_id`, p.`tenant_id`) IS NOT NULL
              AND audit_t.`id` <> COALESCE(b.`tenant_id`, p.`tenant_id`)
           THEN 'audit user and parent owners conflict'
         WHEN audit_t.`id` IS NULL
              AND COALESCE(b.`tenant_id`, p.`tenant_id`) IS NULL
           THEN 'audit user and bid/project owner are unresolved'
         ELSE 'tenant_id remained NULL'
       END
FROM `audit_task` AS a
LEFT JOIN `tenant` AS audit_t
  ON audit_t.`tenant_code` = CONCAT('user-', a.`audit_user_id`)
LEFT JOIN `bid_document` AS b
  ON b.`id` = a.`bid_id`
LEFT JOIN `project` AS p
  ON p.`id` = b.`project_id`
WHERE a.`tenant_id` IS NULL
UNION ALL
SELECT 'audit_issue', CAST(i.`id` AS CHAR), CAST(i.`audit_id` AS CHAR),
       'parent audit_task is missing or unresolved'
FROM `audit_issue` AS i
LEFT JOIN `audit_task` AS a ON a.`id` = i.`audit_id`
WHERE i.`tenant_id` IS NULL AND (a.`id` IS NULL OR a.`tenant_id` IS NULL)
UNION ALL
SELECT 'audit_report', CAST(r.`id` AS CHAR), CAST(r.`audit_id` AS CHAR),
       'parent audit_task is missing or unresolved'
FROM `audit_report` AS r
LEFT JOIN `audit_task` AS a ON a.`id` = r.`audit_id`
WHERE r.`tenant_id` IS NULL AND (a.`id` IS NULL OR a.`tenant_id` IS NULL)
UNION ALL
SELECT 'audit_task_event', CAST(e.`id` AS CHAR), e.`task_id`,
       'parent audit_task is missing or unresolved'
FROM `audit_task_event` AS e
LEFT JOIN `audit_task` AS a ON a.`task_id` = e.`task_id`
WHERE e.`tenant_id` IS NULL AND (a.`id` IS NULL OR a.`tenant_id` IS NULL)
UNION ALL
SELECT 'knowledge_file', CAST(k.`id` AS CHAR), CAST(k.`upload_user_id` AS CHAR),
       CASE
         WHEN k.`upload_user_id` IS NULL THEN 'upload_user_id is NULL'
         ELSE 'personal tenant for upload_user_id is missing'
       END
FROM `knowledge_file` AS k
LEFT JOIN `tenant` AS t
  ON t.`tenant_code` = CONCAT('user-', k.`upload_user_id`)
WHERE k.`tenant_id` IS NULL AND (k.`upload_user_id` IS NULL OR t.`id` IS NULL)
UNION ALL
SELECT 'knowledge_chunk', CAST(c.`id` AS CHAR), CAST(c.`file_id` AS CHAR),
       'parent knowledge_file is missing or unresolved'
FROM `knowledge_chunk` AS c
LEFT JOIN `knowledge_file` AS k ON k.`id` = c.`file_id`
WHERE c.`tenant_id` IS NULL AND (k.`id` IS NULL OR k.`tenant_id` IS NULL)
UNION ALL
SELECT 'chat_message', CAST(m.`id` AS CHAR), CAST(m.`user_id` AS CHAR),
       CASE
         WHEN m.`user_id` IS NULL THEN 'user_id is NULL'
         ELSE 'personal tenant for user_id is missing'
       END
FROM `chat_message` AS m
LEFT JOIN `tenant` AS t
  ON t.`tenant_code` = CONCAT('user-', m.`user_id`)
WHERE m.`tenant_id` IS NULL AND (m.`user_id` IS NULL OR t.`id` IS NULL)
UNION ALL
SELECT 'document_parse_job', CAST(j.`id` AS CHAR), CAST(j.`file_id` AS CHAR),
       'parent bid_document is missing or unresolved'
FROM `document_parse_job` AS j
LEFT JOIN `bid_document` AS b ON b.`id` = j.`file_id`
WHERE j.`tenant_id` IS NULL AND (b.`id` IS NULL OR b.`tenant_id` IS NULL)
UNION ALL
SELECT 'rag_trigger_outbox', CAST(o.`id` AS CHAR),
       CONCAT('job_id=', o.`job_id`, '; file_id=', o.`file_id`),
       CASE
         WHEN j.`tenant_id` IS NOT NULL AND b.`tenant_id` IS NOT NULL
              AND j.`tenant_id` <> b.`tenant_id` THEN 'job and bid owners conflict'
         ELSE 'parse job and bid_document owners are unresolved'
       END
FROM `rag_trigger_outbox` AS o
LEFT JOIN `document_parse_job` AS j ON j.`job_id` = o.`job_id`
LEFT JOIN `bid_document` AS b ON b.`id` = o.`file_id`
WHERE o.`tenant_id` IS NULL
  AND (
    (j.`tenant_id` IS NULL AND b.`tenant_id` IS NULL)
    OR (
      j.`tenant_id` IS NOT NULL
      AND b.`tenant_id` IS NOT NULL
      AND j.`tenant_id` <> b.`tenant_id`
    )
  )
ORDER BY `source_table`, `source_id`;

-- 7. Parent/child tenant mismatches must be empty before Enforce.
SELECT 'bid_document/project' AS `relationship`, CAST(b.`id` AS CHAR) AS `source_id`,
       'bid_document tenant differs from project tenant' AS `reason`
FROM `bid_document` AS b
JOIN `project` AS p ON p.`id` = b.`project_id`
WHERE b.`tenant_id` IS NOT NULL AND p.`tenant_id` IS NOT NULL
  AND b.`tenant_id` <> p.`tenant_id`
UNION ALL
SELECT 'audit_task/bid_document', CAST(a.`id` AS CHAR),
       'audit_task tenant differs from bid_document tenant'
FROM `audit_task` AS a
JOIN `bid_document` AS b ON b.`id` = a.`bid_id`
WHERE a.`tenant_id` IS NOT NULL AND b.`tenant_id` IS NOT NULL
  AND a.`tenant_id` <> b.`tenant_id`
UNION ALL
SELECT 'audit_issue/audit_task', CAST(i.`id` AS CHAR),
       'audit_issue tenant differs from audit_task tenant'
FROM `audit_issue` AS i
JOIN `audit_task` AS a ON a.`id` = i.`audit_id`
WHERE i.`tenant_id` IS NOT NULL AND a.`tenant_id` IS NOT NULL
  AND i.`tenant_id` <> a.`tenant_id`
UNION ALL
SELECT 'audit_report/audit_task', CAST(r.`id` AS CHAR),
       'audit_report tenant differs from audit_task tenant'
FROM `audit_report` AS r
JOIN `audit_task` AS a ON a.`id` = r.`audit_id`
WHERE r.`tenant_id` IS NOT NULL AND a.`tenant_id` IS NOT NULL
  AND r.`tenant_id` <> a.`tenant_id`
UNION ALL
SELECT 'audit_task_event/audit_task', CAST(e.`id` AS CHAR),
       'audit_task_event tenant differs from audit_task tenant'
FROM `audit_task_event` AS e
JOIN `audit_task` AS a ON a.`task_id` = e.`task_id`
WHERE e.`tenant_id` IS NOT NULL AND a.`tenant_id` IS NOT NULL
  AND e.`tenant_id` <> a.`tenant_id`
UNION ALL
SELECT 'knowledge_chunk/knowledge_file', CAST(c.`id` AS CHAR),
       'knowledge_chunk tenant differs from knowledge_file tenant'
FROM `knowledge_chunk` AS c
JOIN `knowledge_file` AS k ON k.`id` = c.`file_id`
WHERE c.`tenant_id` IS NOT NULL AND k.`tenant_id` IS NOT NULL
  AND c.`tenant_id` <> k.`tenant_id`
UNION ALL
SELECT 'document_parse_job/bid_document', CAST(j.`id` AS CHAR),
       'document_parse_job tenant differs from bid_document tenant'
FROM `document_parse_job` AS j
JOIN `bid_document` AS b ON b.`id` = j.`file_id`
WHERE j.`tenant_id` IS NOT NULL AND b.`tenant_id` IS NOT NULL
  AND j.`tenant_id` <> b.`tenant_id`
UNION ALL
SELECT 'rag_trigger_outbox/parent', CAST(o.`id` AS CHAR),
       'rag_trigger_outbox tenant differs from resolved parent tenant'
FROM `rag_trigger_outbox` AS o
LEFT JOIN `document_parse_job` AS j ON j.`job_id` = o.`job_id`
LEFT JOIN `bid_document` AS b ON b.`id` = o.`file_id`
WHERE o.`tenant_id` IS NOT NULL
  AND (
    (j.`tenant_id` IS NOT NULL AND o.`tenant_id` <> j.`tenant_id`)
    OR (j.`tenant_id` IS NULL AND b.`tenant_id` IS NOT NULL AND o.`tenant_id` <> b.`tenant_id`)
  )
ORDER BY `relationship`, `source_id`;
