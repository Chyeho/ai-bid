USE `smart_tender_system`;

-- 1. 插入用户数据 (密码为MD5加密的 123456)
INSERT INTO `sys_user` (`id`, `username`, `password`, `real_name`, `email`, `phone`, `status`, `create_time`, `update_time`) VALUES
(1, 'admin', 'e10adc3949ba59abbe56e057f20f883e', '系统管理员', 'admin@ithsd.com', '13800138000', 1, NOW(), NOW()),
(2, 'auditor', 'e10adc3949ba59abbe56e057f20f883e', '资深审核员', 'auditor@ithsd.com', '13900139000', 1, NOW(), NOW()),
(3, 'manager', 'e10adc3949ba59abbe56e057f20f883e', '项目经理', 'manager@ithsd.com', '13700137000', 1, NOW(), NOW());

-- 2. 插入项目数据
INSERT INTO `project` (`id`, `user_id`, `project_name`, `supplier_name`, `parse_status`, `latest_version`, `create_time`, `update_time`) VALUES
(1001, 2, '2024年度数据中心服务器采购项目', '华为技术有限公司', 1, 2, DATE_SUB(NOW(), INTERVAL 5 DAY), DATE_SUB(NOW(), INTERVAL 1 DAY)),
(1002, 2, '高新区智慧园区弱电智能化工程', '中建三局', 0, 1, DATE_SUB(NOW(), INTERVAL 3 DAY), DATE_SUB(NOW(), INTERVAL 3 DAY)),
(1003, 3, '集团总部办公用品年度框架协议采购', '得力集团', 0, 1, NOW(), NOW()),
(1004, 3, '2024年秋季校园绿化改造工程', '绿城园林', 1, 1, DATE_SUB(NOW(), INTERVAL 10 DAY), DATE_SUB(NOW(), INTERVAL 8 DAY));

-- 3. 插入标书文件数据 (包含不同版本)
INSERT INTO `bid_document` (`id`, `file_name`, `file_path`, `file_size`, `file_type`, `file_category`, `bid_name`, `supplier_name`, `budget_amount`, `page_count`, `parse_status`, `upload_user_id`, `upload_time`, `version`, `project_id`) VALUES
(1, '2024年度服务器采购项目标书_v1.docx', '/upload/files/20240301/server_purchase_hw_v1.docx', 10485760, 'word', 'bid', '2024年度数据中心服务器采购项目', '华为技术有限公司', 5000000.00, 120, 2, 2, DATE_SUB(NOW(), INTERVAL 5 DAY), 1, 1001),
(2, '2024年度服务器采购项目标书_v2.docx', '/upload/files/20240304/server_purchase_hw_v2.docx', 10585760, 'word', 'bid', '2024年度数据中心服务器采购项目', '华为技术有限公司', 5000000.00, 125, 2, 2, DATE_SUB(NOW(), INTERVAL 1 DAY), 2, 1001),
(3, '智慧园区弱电工程投标文件_中建三局.pdf', '/upload/files/20240302/smart_park_construction.pdf', 25165824, 'pdf', 'bid', '高新区智慧园区弱电智能化工程', '中建三局', 12000000.00, 350, 1, 2, DATE_SUB(NOW(), INTERVAL 3 DAY), 1, 1002),
(4, '办公用品年度框架协议采购.docx', '/upload/files/20240303/office_supplies.docx', 5242880, 'word', 'bid', '集团总部办公用品年度框架协议采购', '得力集团', 800000.00, 50, 0, 3, NOW(), 1, 1003),
(5, '2024年秋季校园绿化改造工程标书.pdf', '/upload/files/20240220/campus_greening.pdf', 15165824, 'pdf', 'bid', '2024年秋季校园绿化改造工程', '绿城园林', 2000000.00, 80, 2, 3, DATE_SUB(NOW(), INTERVAL 10 DAY), 1, 1004);

-- 4. 插入审核任务数据
INSERT INTO `audit_task` (`id`, `task_id`, `bid_id`, `task_status`, `audit_result`, `issue_count`, `critical_count`, `warning_count`, `info_count`, `start_time`, `end_time`, `audit_user_id`, `create_time`) VALUES
(1, 'TASK-20240301-001', 1, 2, 'revise', 5, 1, 2, 2, DATE_SUB(NOW(), INTERVAL 118 HOUR), DATE_SUB(NOW(), INTERVAL 116 HOUR), 2, DATE_SUB(NOW(), INTERVAL 118 HOUR)),
(2, 'TASK-20240304-001', 2, 2, 'pass', 1, 0, 0, 1, DATE_SUB(NOW(), INTERVAL 22 HOUR), DATE_SUB(NOW(), INTERVAL 20 HOUR), 2, DATE_SUB(NOW(), INTERVAL 22 HOUR)),
(3, 'TASK-20240302-001', 3, 1, NULL, 12, 1, 5, 6, DATE_SUB(NOW(), INTERVAL 10 HOUR), NULL, 2, DATE_SUB(NOW(), INTERVAL 10 HOUR)),
(4, 'TASK-20240220-001', 5, 2, 'pass', 0, 0, 0, 0, DATE_SUB(NOW(), INTERVAL 8 DAY), DATE_SUB(NOW(), INTERVAL 7 DAY), 3, DATE_SUB(NOW(), INTERVAL 8 DAY));

-- 5. 插入审核报告数据
INSERT INTO `audit_report` (`id`, `audit_id`, `doc_content`, `version`, `generate_time`) VALUES
(1, 1, '# 审核报告 (V1)\n## 项目：2024年度数据中心服务器采购项目\n### 审核结果：需修改\n发现关键条款遗漏，需补充售后服务说明。', 1, DATE_SUB(NOW(), INTERVAL 116 HOUR)),
(2, 2, '# 审核报告 (V2)\n## 项目：2024年度数据中心服务器采购项目\n### 审核结果：通过\n已根据V1意见修改，符合招标要求。', 2, DATE_SUB(NOW(), INTERVAL 20 HOUR)),
(3, 4, '# 审核报告\n## 项目：2024年秋季校园绿化改造工程\n### 审核结果：通过\n报价合理，资质齐全。', 1, DATE_SUB(NOW(), INTERVAL 7 DAY));

-- 6. 插入审核问题数据
INSERT INTO `audit_issue` (`audit_id`, `issue_no`, `severity`, `category`, `description`, `suggestion`, `page_number`, `section_name`, `context`, `reference`, `create_time`) VALUES
(1, 'ISSUE-001', 'critical', 'clause', '缺少原厂售后服务承诺函', '请补充华为原厂开具的三年质保承诺函', 45, '售后服务', '提供三年免费质保', '招标文件要求：必须提供原厂质保承诺函', DATE_SUB(NOW(), INTERVAL 117 HOUR)),
(3, 'ISSUE-002', 'critical', 'legal', '投标有效期不符合招标文件要求', '建议修改投标有效期为90天', 15, '商务条款', '投标有效期：60天', '招标文件第二章3.1条：投标有效期不得少于90天', DATE_SUB(NOW(), INTERVAL 9 HOUR)),
(3, 'ISSUE-003', 'warning', 'price', '分项报价合计与总价不符', '请重新核对分项报价表', 108, '报价一览表', '分项合计：11,900,000.00元，总报价：12,000,000.00元', '招标文件第三章：报价计算规则', DATE_SUB(NOW(), INTERVAL 9 HOUR)),
(3, 'ISSUE-004', 'warning', 'format', '目录页码与实际内容不对应', '更新目录页码', 3, '目录', '第三章 技术方案 ... P45', '实际第三章起始页为P48', DATE_SUB(NOW(), INTERVAL 8 HOUR)),
(3, 'ISSUE-005', 'info', 'clause', '质保期表述模糊', '明确质保期起算时间', 56, '售后服务承诺', '质保期：验收合格后3年', '建议明确为“最终验收合格签字之日起3年”', DATE_SUB(NOW(), INTERVAL 8 HOUR));

-- 7. 插入知识库文件数据
INSERT INTO `knowledge_file` (`id`, `file_name`, `file_path`, `file_size`, `file_type`, `category`, `tags`, `description`, `status`, `version`, `chunk_count`, `upload_user_id`, `upload_time`, `update_time`) VALUES
(1, '中华人民共和国政府采购法.pdf', '/knowledge/laws/gov_purchase_law.pdf', 1024000, 'pdf', 'law', '法律,政府采购', '政府采购基础法律法规', 1, 1, 50, 1, DATE_SUB(NOW(), INTERVAL 10 DAY), DATE_SUB(NOW(), INTERVAL 10 DAY)),
(2, '建筑工程施工质量验收统一标准.pdf', '/knowledge/standards/construction_quality.pdf', 2048000, 'pdf', 'standard', '建筑,国标', 'GB 50300-2013', 1, 1, 120, 1, DATE_SUB(NOW(), INTERVAL 5 DAY), DATE_SUB(NOW(), INTERVAL 5 DAY));

-- 8. 插入知识库分块数据 (示例)
INSERT INTO `knowledge_chunk` (`file_id`, `chunk_index`, `chunk_text`, `chunk_length`, `vector_id`, `page_number`, `section_name`, `create_time`) VALUES
(1, 1, '第一条 为了规范政府采购行为，提高政府采购资金的使用效益，维护国家利益和社会公共利益，保护政府采购当事人的合法权益，促进廉政建设，制定本法。', 80, 'vec_001', 1, '第一章 总则', NOW()),
(1, 2, '第二条 在中华人民共和国境内进行的政府采购适用本法。本法所称政府采购，是指各级国家机关、事业单位和团体组织，使用财政性资金采购依法制定的集中采购目录以内的或者采购限额标准以上的货物、工程和服务的行为。', 120, 'vec_002', 1, '第一章 总则', NOW());

-- 9. 插入 AI 对话消息数据
INSERT INTO `chat_message` (`id`, `project_id`, `bid_id`, `user_id`, `role`, `content`, `create_time`) VALUES
(1, 1001, 1, 2, 'user', '请问这份标书中关于售后服务的要求，符合通用标准吗？', DATE_SUB(NOW(), INTERVAL 117 HOUR)),
(2, 1001, 1, 2, 'assistant', '根据招标文件要求，必须提供原厂质保承诺函。当前标书仅说明“提供三年免费质保”，缺少原厂证明，存在风险。', DATE_SUB(NOW(), INTERVAL 117 HOUR)),
(3, 1002, 3, 2, 'user', '帮我检查一下报价表里的数据计算有没有问题。', DATE_SUB(NOW(), INTERVAL 9 HOUR)),
(4, 1002, 3, 2, 'assistant', '我发现了计算错误：分项合计为 11,900,000.00 元，但总报价填写为 12,000,000.00 元，两者不符，建议仔细核对。', DATE_SUB(NOW(), INTERVAL 9 HOUR));
