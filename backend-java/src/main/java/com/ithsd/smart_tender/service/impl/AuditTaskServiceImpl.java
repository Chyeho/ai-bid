package com.ithsd.smart_tender.service.impl;

import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.ithsd.smart_tender.context.BaseContext;
import com.ithsd.smart_tender.exception.BizException;
import com.ithsd.smart_tender.mapper.AuditTaskMapper;
import com.ithsd.smart_tender.mapper.KnowledgeFileMapper;
import com.ithsd.smart_tender.mapper.TenderMapper;
import com.ithsd.smart_tender.pojo.dto.CreateAuditTaskRequest;
import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.entity.AuditTask;
import com.ithsd.smart_tender.pojo.entity.AuditTaskEntity;
import com.ithsd.smart_tender.pojo.entity.KnowledgeFile;
import com.ithsd.smart_tender.pojo.entity.Tender;
import com.ithsd.smart_tender.pojo.enums.AuditStageEnum;
import com.ithsd.smart_tender.pojo.enums.AuditTaskStatusEnum;
import com.ithsd.smart_tender.pojo.enums.SseEventTypeEnum;
import com.ithsd.smart_tender.pojo.vo.AuditTaskCreateVO;
import com.ithsd.smart_tender.pojo.vo.AuditTaskStatusVO;
import com.ithsd.smart_tender.pojo.vo.IssueVO;
import com.ithsd.smart_tender.pojo.vo.ResultVO;
import com.ithsd.smart_tender.pojo.vo.SummaryVO;
import com.ithsd.smart_tender.repository.AuditIssueRepository;
import com.ithsd.smart_tender.repository.AuditTaskRepository;
import com.ithsd.smart_tender.service.AuditTaskService;
import com.ithsd.smart_tender.service.TenderService;
import com.ithsd.smart_tender.service.queue.AuditTaskDispatcher;
import com.ithsd.smart_tender.service.trigger.RagTriggerOutboxService;
import com.ithsd.smart_tender.sse.AuditTaskEventService;
import com.ithsd.smart_tender.sse.ReplaySseEvent;
import com.ithsd.smart_tender.sse.SseHub;
import lombok.RequiredArgsConstructor;
import lombok.extern.slf4j.Slf4j;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.PageRequest;
import org.springframework.data.domain.Pageable;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;
import org.springframework.web.servlet.mvc.method.annotation.SseEmitter;
import java.time.DayOfWeek;
import java.time.LocalDate;
import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;
import java.time.temporal.TemporalAdjusters;
import java.util.*;

/**
 * 审核任务管理的业务实现（Service 实现类）。
 *   <li>任务生命周期的“管理接口”：创建任务、查询状态、查询结果。</li>
 *   <li>与前端交互相关的“推送入口”：建立 SSE 连接、断线重放（replay）。</li>
 */
@Service
@Slf4j
public class AuditTaskServiceImpl implements AuditTaskService {

    private final AuditTaskMapper auditTaskMapper;
    private final TenderService tenderService;

    /**
     * 审核任务表的数据库访问入口（JPA Repository）。
     */
    private final AuditTaskRepository auditTaskRepository;
    /**
     * 审核问题表的数据库访问入口（JPA Repository）。
     */
    private final AuditIssueRepository auditIssueRepository;
    /**
     * 任务调度器：负责把 taskId 投递到某种“执行通道”。
     *
     * <p>具体通道由配置决定：</p>
     * <ul>
     *   <li>async：直接触发引擎异步执行</li>
     *   <li>redis-list：入队到 Redis List，交给 worker 轮询消费</li>
     *   <li>redis-stream：写入 Redis Stream，交给 consumer group 消费</li>
     * </ul>
     */
    private final AuditTaskDispatcher taskDispatcher;
    /**
     * SSE 事件持久化与回放服务：用于“断线重连”补发事件。
     */
    private final AuditTaskEventService eventService;
    /**
     * SSE 连接管理与广播：一个 taskId 可能对应多个前端订阅连接。
     */
    private final SseHub sseHub;
    private final RagTriggerOutboxService ragTriggerOutboxService;
    private final TenderMapper tenderMapper;
    private final KnowledgeFileMapper knowledgeFileMapper;

    public AuditTaskServiceImpl(
            AuditTaskMapper auditTaskMapper, TenderService tenderService, AuditTaskRepository auditTaskRepository,
            AuditIssueRepository auditIssueRepository,
            AuditTaskDispatcher taskDispatcher,
            AuditTaskEventService eventService,
            SseHub sseHub,
            RagTriggerOutboxService ragTriggerOutboxService,
            TenderMapper tenderMapper,
            KnowledgeFileMapper knowledgeFileMapper
    ) {
        this.auditTaskMapper = auditTaskMapper;
        this.tenderService = tenderService;
        this.auditTaskRepository = auditTaskRepository;
        this.auditIssueRepository = auditIssueRepository;
        this.taskDispatcher = taskDispatcher;
        this.eventService = eventService;
        this.sseHub = sseHub;
        this.ragTriggerOutboxService = ragTriggerOutboxService;
        this.tenderMapper = tenderMapper;
        this.knowledgeFileMapper = knowledgeFileMapper;
    }

    @Override
    @Transactional
    public AuditTaskCreateVO createTask(CreateAuditTaskRequest request) {
        // createTask 是“写操作”：需要事务保证任务记录写入成功
        LocalDateTime now = LocalDateTime.now();
        AuditTaskEntity entity = new AuditTaskEntity();
        // taskId：业务 ID（对外查询/订阅使用），不要与数据库自增 id 混淆
        entity.setTaskId(buildTaskId());
        // bidId：标书/投标文件 ID，决定本次审核的输入来源
        entity.setBidId(request.getBidId());
        // 初始状态：待处理（pending），等待引擎/worker 执行
        entity.setTaskStatus(AuditTaskStatusEnum.PENDING.getCode());
        // 初始阶段：从“文档抽取”开始（实际执行由引擎推进）
        entity.setStage(AuditStageEnum.DOC_EXTRACT.name());
        entity.setProgress(0);
        // enabledChecks：本次启用的检查项，不传默认全启用
        entity.setEnabledChecks(request.enabledChecksOrDefault());
        entity.setIssueCount(0);
        Long auditUserId = BaseContext.getCurrentId();
        if (auditUserId == null && request.getBidId() != null) {
            Tender tender = tenderMapper.selectById(request.getBidId());
            if (tender != null) {
                auditUserId = tender.getUploadUserId();
            }
        }
        entity.setAuditUserId(auditUserId);
        entity.setFailedStages(new ArrayList<>());
        entity.setErrorMsg(null);
        entity.setStartTime(null);
        entity.setEndTime(null);
        entity.setCreateTime(now);
        entity.setUpdatedAt(now);
        AuditTaskEntity saved = auditTaskRepository.save(entity);

        // 写入 RAG 触发表，后续由 RagTriggerOutboxDispatcher 定时拉取并调用 Python
        boolean webSearchEnabled = Boolean.TRUE.equals(request.getWebSearchEnabled());
        boolean forceRefresh = Boolean.TRUE.equals(request.getForceRefresh());
        String strategyVersion = webSearchEnabled ? "audit-websearch" : "default";
        if (forceRefresh) {
            strategyVersion = strategyVersion + "|force-refresh";
        }
        ragTriggerOutboxService.enqueue(
                saved.getBidId(),
                0,
                strategyVersion,
                "",              // 审核任务不做去重哈希，直接传空字符串
                saved.getTaskId()   // jobId 使用 taskId，用于区分审核任务 vs 文件入库
        );

        return new AuditTaskCreateVO(saved.getTaskId());
    }

    @Override
    @Transactional(readOnly = true)
    public AuditTaskStatusVO getStatus(String taskId) {
        // readOnly=true：表示只读事务（优化，避免不必要的写锁/脏检查）
        AuditTaskEntity task = loadTask(taskId);
        AuditTaskStatusVO statusVO = new AuditTaskStatusVO();
        statusVO.setTaskId(task.getTaskId());
        // taskStatus 在数据库里是数字码，这里转换成更“前端友好”的字符串（pending/processing/...）
        statusVO.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        statusVO.setStage(task.getStage());
        statusVO.setProgress(task.getProgress());
        statusVO.setIssueCount(task.getIssueCount());
        statusVO.setFailedStages(task.getFailedStages() == null ? List.of() : task.getFailedStages());
        statusVO.setTotalFileCount(auditTaskRepository.count());
        statusVO.setPendingFileCount(defaultIfNull(auditTaskRepository.countByTaskStatus(AuditTaskStatusEnum.PENDING.getCode())));
        statusVO.setProcessingFileCount(defaultIfNull(auditTaskRepository.countByTaskStatus(AuditTaskStatusEnum.PROCESSING.getCode())));
        statusVO.setFailedFileCount(defaultIfNull(auditTaskRepository.countByTaskStatus(AuditTaskStatusEnum.FAILED.getCode())));
        return statusVO;
    }

    @Override
    @Transactional(readOnly = true)
    public ResultVO getResult(String taskId, Integer page, Integer size, String sinceIssueNo) {
        AuditTaskEntity task = loadTask(taskId);
        // PageRequest：分页参数（注意 page 从 0 开始，所以外部传 1 时这里要减 1）
        Pageable pageable = PageRequest.of(page - 1, size);
        Page<AuditIssueEntity> issuePage;
        if (StringUtils.hasText(sinceIssueNo)) {
            // 增量拉取：只返回 issueNo 大于某个值的记录（适合“不断追加问题”的场景）
            issuePage = auditIssueRepository.findByAuditIdAndIssueNoGreaterThanOrderByIssueNoAsc(task.getId(), sinceIssueNo, pageable);
        } else {
            issuePage = auditIssueRepository.findByAuditIdOrderByIssueNoAsc(task.getId(), pageable);
        }

        ResultVO resultVO = new ResultVO();
        resultVO.setTaskId(task.getTaskId());
        // totalIssues：总问题数（用于汇总显示）
        Long totalIssues = defaultIfNull(auditIssueRepository.countByAuditId(task.getId()));
        resultVO.setAuditResult(resolveAuditResult(task, totalIssues));

        SummaryVO summaryVO = new SummaryVO();
        summaryVO.setTotalIssues(totalIssues.intValue());
        // 严重级别统计：critical/warning/info（用于前端展示汇总卡片）
        summaryVO.setCritical(defaultIfNull(auditIssueRepository.countByAuditIdAndSeverity(task.getId(), "critical")).intValue());
        summaryVO.setWarning(defaultIfNull(auditIssueRepository.countByAuditIdAndSeverity(task.getId(), "warning")).intValue());
        summaryVO.setInfo(defaultIfNull(auditIssueRepository.countByAuditIdAndSeverity(task.getId(), "info")).intValue());
        resultVO.setSummary(summaryVO);
        resultVO.setIssues(issuePage.getContent().stream().map(this::toIssueVO).toList());
        return resultVO;
    }

    @Override
    @Transactional(readOnly = true)
    public SseEmitter subscribeStream(String taskId, String lastEventId) {
        // 订阅时先拿一次当前状态，保证前端“刚连上就有状态可画”
        AuditTaskStatusVO statusVO = getStatus(taskId);
        // SseEmitter：Spring 对 SSE 的封装（相当于一条到浏览器的推送通道）
        SseEmitter emitter = sseHub.subscribe(taskId);
        if (StringUtils.hasText(lastEventId)) {
            // 断线重连：回放 lastEventId 之后的事件，避免前端漏掉 progress/issue/complete
            replay(taskId, lastEventId, emitter);
        } else {
            try {
                // 首次订阅：先推一条 progress，让前端立即渲染当前进度
                sseHub.emitToEmitter(emitter, SseEventTypeEnum.PROGRESS, statusVO, null);
            } catch (RuntimeException | java.io.IOException ignored) {
            }
        }
        return emitter;
    }

    @Override
    public List<Long> getAuditIdsByBidIds(List<Long> bidIds) {
        if(bidIds.isEmpty())
            return List.of();
        // 一个标书对应可能不止对应审核报告
        LambdaQueryWrapper<AuditTask> queryWrapper = new LambdaQueryWrapper<AuditTask>()
                .select(AuditTask::getId)
                .in(AuditTask::getBidId, bidIds);
        return auditTaskMapper.selectObjs(queryWrapper)
                .stream()
                // 安全转换：先判断非空+类型匹配，再转换
                .filter(Objects::nonNull)
                .filter(obj -> obj instanceof Long)
                .map(obj -> (Long) obj)
                .toList();
    }

    @Override
    public Map<String, Long> countByWeek() {
        Long userId = BaseContext.getCurrentId();
        // 初始化本周的7天数据，默认值为0
        Map<String, Long> result = new LinkedHashMap<>();
        result.put("Monday", 0L);
        result.put("Tuesday", 0L);
        result.put("Wednesday", 0L);
        result.put("Thursday", 0L);
        result.put("Friday", 0L);
        result.put("Saturday", 0L);
        result.put("Sunday", 0L);

        if (userId == null) {
            return result;
        }

        List<Long> bidIds = tenderService.getBidIdsByUserId(userId);
        
        // 如果该用户没有标书，直接返回初始化的Map
        if (bidIds.isEmpty()) {
            return result;
        }

        // 统计本周每天的审核报告数
        List<Map<String, Object>> counts = auditTaskMapper.countByWeek(bidIds);
        
        // 获取本周一的日期
        LocalDate today = LocalDate.now();
        LocalDate monday = today.with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY));
        
        // 映射日期到星期几
        DateTimeFormatter formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd");
        Map<String, String> dateToDayMap = new HashMap<>();
        for (int i = 0; i < 7; i++) {
            LocalDate date = monday.plusDays(i);
            String dateStr = date.format(formatter);
            String dayName = date.getDayOfWeek().name();
            // 首字母大写，其余小写
            dayName = dayName.substring(0, 1).toUpperCase() + dayName.substring(1).toLowerCase();
            dateToDayMap.put(dateStr, dayName);
        }

        for (Map<String, Object> map : counts) {
            String date = (String) map.get("day_date");
            Long count = ((Number) map.get("count")).longValue();
            
            // 如果查询结果的日期在本周内，则更新对应的星期计数
            String dayName = dateToDayMap.get(date);
            if (dayName != null) {
                result.put(dayName, count);
            }
        }
        return result;
    }

    private void replay(String taskId, String lastEventId, SseEmitter emitter) {
        List<ReplaySseEvent> replayEvents = eventService.replay(taskId, lastEventId);
        for (ReplaySseEvent replayEvent : replayEvents) {
            try {
                sseHub.emitToEmitter(emitter, replayEvent.getEventType(), replayEvent.getData(), replayEvent.getEventId());
            } catch (RuntimeException | java.io.IOException ignored) {
                break;
            }
        }
    }

    @Override
    @Transactional
    public void markTaskProcessing(String taskId) {
        AuditTaskEntity task;
        try {
            task = loadTask(taskId);
        } catch (Exception e) {
            log.warn("标记任务处理中失败，任务不存在 taskId={}", taskId);
            return;
        }
        if (AuditTaskStatusEnum.COMPLETED.getCode().equals(task.getTaskStatus())
                || AuditTaskStatusEnum.FAILED.getCode().equals(task.getTaskStatus())) {
            return;
        }
        task.setTaskStatus(AuditTaskStatusEnum.PROCESSING.getCode());
        task.setStage(AuditStageEnum.RAG.name());
        if (task.getProgress() == null || task.getProgress() < 5) {
            task.setProgress(5);
        }
        if (task.getStartTime() == null) {
            task.setStartTime(LocalDateTime.now());
        }
        task.setUpdatedAt(LocalDateTime.now());
        auditTaskRepository.save(task);
        emitSafe(taskId, SseEventTypeEnum.PROGRESS, getStatus(taskId));
    }

    @Override
    @Transactional
    public void markTaskFailed(String taskId, String errorMessage) {
        AuditTaskEntity task;
        try {
            task = loadTask(taskId);
        } catch (Exception e) {
            log.warn("标记任务失败时任务不存在 taskId={}", taskId);
            return;
        }
        if (AuditTaskStatusEnum.COMPLETED.getCode().equals(task.getTaskStatus())) {
            return;
        }
        task.setTaskStatus(AuditTaskStatusEnum.FAILED.getCode());
        task.setErrorMsg(StringUtils.hasText(errorMessage) ? errorMessage : "触发审核失败");
        if (task.getStage() == null) {
            task.setStage(AuditStageEnum.RAG.name());
        }
        if (task.getProgress() == null) {
            task.setProgress(0);
        }
        task.setEndTime(LocalDateTime.now());
        task.setUpdatedAt(LocalDateTime.now());
        auditTaskRepository.save(task);
        emitSafe(taskId, SseEventTypeEnum.PROGRESS, getStatus(taskId));
    }

    @Override
    @Transactional
    public void processAuditResult(String taskId, String responseBody) {
        if (!StringUtils.hasText(responseBody)) {
            return;
        }
        AuditTaskEntity task;
        try {
            task = loadTask(taskId);
        } catch (Exception e) {
            log.error("无法处理审核结果，找不到任务 taskId={}", taskId);
            return;
        }

        try {
            com.alibaba.fastjson2.JSONObject root = com.alibaba.fastjson2.JSON.parseObject(responseBody);
            com.alibaba.fastjson2.JSONArray issuesArray = extractIssuesArray(root);
            if (issuesArray == null) {
                issuesArray = new com.alibaba.fastjson2.JSONArray();
            }

            int issueCount = 0;
            List<AuditIssueEntity> parsedIssues = new ArrayList<>();
            List<String> issueAnchorKeys = new ArrayList<>();
            Map<String, Map<Integer, Integer>> anchorPageVotes = new LinkedHashMap<>();
            for (int i = 0; i < issuesArray.size(); i++) {
                com.alibaba.fastjson2.JSONObject issueObj = issuesArray.getJSONObject(i);
                if (issueObj == null) {
                    continue;
                }
                AuditIssueEntity issue = new AuditIssueEntity();
                issue.setAuditId(task.getId());
                issue.setIssueNo("ISSUE-" + String.format("%03d", i + 1));
                issue.setSeverity(issueObj.getString("severity"));
                issue.setCategory(resolveCategory(issueObj));
                issue.setDescription(buildIssueDescription(issueObj));

                com.alibaba.fastjson2.JSONArray suggestions = issueObj.getJSONArray("suggestions");
                if (suggestions != null && !suggestions.isEmpty()) {
                    List<String> suggestionTexts = new ArrayList<>();
                    for (int j = 0; j < suggestions.size(); j++) {
                        Object suggestionItem = suggestions.get(j);
                        if (suggestionItem == null) {
                            continue;
                        }
                        if (suggestionItem instanceof String) {
                            suggestionTexts.add((String) suggestionItem);
                        } else {
                            suggestionTexts.add(String.valueOf(suggestionItem));
                        }
                    }
                    issue.setSuggestion(String.join("\n", suggestionTexts));
                }

                com.alibaba.fastjson2.JSONArray citations = issueObj.getJSONArray("citations");
                com.alibaba.fastjson2.JSONObject firstCitation = null;
                if (citations != null && !citations.isEmpty()) {
                    firstCitation = citations.getJSONObject(0);
                    issue.setContext(firstCitation.getString("text"));
                }
                issue.setPageNumber(resolveBestPageNumber(citations, task.getBidId()));
                issue.setSectionName(resolveBestSectionName(citations, firstCitation, task.getBidId()));
                issue.setReference(resolveReference(issueObj, citations, firstCitation, task.getBidId()));

                issue.setCreateTime(LocalDateTime.now());
                parsedIssues.add(issue);
                String anchorKey = buildAnchorKeyForPageNormalization(issueObj, issue);
                issueAnchorKeys.add(anchorKey);
                Integer page = issue.getPageNumber();
                if (StringUtils.hasText(anchorKey) && page != null && page > 0) {
                    Map<Integer, Integer> votes = anchorPageVotes.computeIfAbsent(anchorKey, k -> new LinkedHashMap<>());
                    votes.put(page, votes.getOrDefault(page, 0) + 1);
                }
                issueCount++;
            }

            Map<String, Integer> canonicalPageByAnchor = new LinkedHashMap<>();
            for (Map.Entry<String, Map<Integer, Integer>> entry : anchorPageVotes.entrySet()) {
                Integer bestPage = null;
                int bestVote = -1;
                for (Map.Entry<Integer, Integer> voteEntry : entry.getValue().entrySet()) {
                    Integer page = voteEntry.getKey();
                    int vote = voteEntry.getValue() == null ? 0 : voteEntry.getValue();
                    if (page == null || page <= 0) {
                        continue;
                    }
                    if (vote > bestVote || (vote == bestVote && (bestPage == null || page < bestPage))) {
                        bestVote = vote;
                        bestPage = page;
                    }
                }
                if (bestPage != null) {
                    canonicalPageByAnchor.put(entry.getKey(), bestPage);
                }
            }

            for (int idx = 0; idx < parsedIssues.size(); idx++) {
                AuditIssueEntity issue = parsedIssues.get(idx);
                String anchorKey = idx < issueAnchorKeys.size() ? issueAnchorKeys.get(idx) : null;
                if (StringUtils.hasText(anchorKey)) {
                    Integer canonicalPage = canonicalPageByAnchor.get(anchorKey);
                    if (canonicalPage != null && canonicalPage > 0) {
                        issue.setPageNumber(canonicalPage);
                    }
                }
                auditIssueRepository.save(issue);
                emitSafe(task.getTaskId(), SseEventTypeEnum.ISSUE, toIssueVO(issue));
            }

            task.setTaskStatus(AuditTaskStatusEnum.COMPLETED.getCode());
            task.setStage(AuditStageEnum.SUMMARY.name());
            task.setProgress(100);
            task.setIssueCount(issueCount);
            task.setAuditResult(issueCount > 0 ? "revise" : "pass");
            task.setEndTime(LocalDateTime.now());
            task.setUpdatedAt(LocalDateTime.now());
            auditTaskRepository.save(task);
            emitSafe(task.getTaskId(), SseEventTypeEnum.COMPLETE, toCompleteVO(task, parsedIssues));
        } catch (Exception e) {
            log.error("解析大模型审查结果失败", e);
            task.setTaskStatus(AuditTaskStatusEnum.FAILED.getCode());
            task.setErrorMsg("解析大模型审查结果失败: " + e.getMessage());
            task.setEndTime(LocalDateTime.now());
            task.setUpdatedAt(LocalDateTime.now());
            auditTaskRepository.save(task);
            emitSafe(task.getTaskId(), SseEventTypeEnum.PROGRESS, getStatus(task.getTaskId()));
        }
    }

    private void emitSafe(String taskId, SseEventTypeEnum eventType, Object payload) {
        try {
            String eventId = eventService.persist(taskId, eventType, payload);
            sseHub.emit(taskId, eventType, payload, eventId);
        } catch (RuntimeException ex) {
            log.warn("emit sse failed, taskId={}, event={}", taskId, eventType.getEventName(), ex);
        }
    }

    private com.alibaba.fastjson2.JSONArray extractIssuesArray(com.alibaba.fastjson2.JSONObject root) {
        if (root == null) {
            return null;
        }
        com.alibaba.fastjson2.JSONArray direct = root.getJSONArray("issues");
        if (direct != null) {
            return direct;
        }
        com.alibaba.fastjson2.JSONObject data = root.getJSONObject("data");
        if (data != null) {
            com.alibaba.fastjson2.JSONArray nested = data.getJSONArray("issues");
            if (nested != null) {
                return nested;
            }
        }
        com.alibaba.fastjson2.JSONObject result = root.getJSONObject("result");
        if (result != null) {
            return result.getJSONArray("issues");
        }
        return null;
    }

    private String resolveCategory(com.alibaba.fastjson2.JSONObject issueObj) {
        String raw = firstNonBlank(issueObj.getString("category"), issueObj.getString("dimension"));
        if (StringUtils.hasText(raw)) {
            raw = raw.trim().toLowerCase();
            if (raw.contains("budget") || raw.contains("预算")) {
                return "budget";
            }
            if (raw.contains("legal") || raw.contains("政策") || raw.contains("法律") || raw.contains("合规")) {
                return "legal";
            }
            if (raw.contains("demand") || raw.contains("需求")) {
                return "demand";
            }
        }
        String title = issueObj.getString("title");
        if (StringUtils.hasText(title)) {
            if (title.contains("预算")) {
                return "budget";
            }
            if (title.contains("政策") || title.contains("法律") || title.contains("合规")) {
                return "legal";
            }
            if (title.contains("需求")) {
                return "demand";
            }
        }
        return "demand";
    }

    private String buildIssueDescription(com.alibaba.fastjson2.JSONObject issueObj) {
        if (issueObj == null) {
            return null;
        }
        String title = issueObj.getString("title");
        String rationale = normalizeIssueRationale(issueObj.getString("rationale"));
        boolean hasTitle = StringUtils.hasText(title);
        boolean hasRationale = StringUtils.hasText(rationale);
        if (hasTitle && hasRationale) {
            return "【问题标题】" + title.trim() + "\n【问题说明】" + rationale.trim();
        }
        if (hasRationale) {
            return rationale.trim();
        }
        if (hasTitle) {
            return title.trim();
        }
        return null;
    }

    private String normalizeIssueRationale(String rationale) {
        if (!StringUtils.hasText(rationale)) {
            return rationale;
        }
        String text = rationale.trim();
        int lastExplain = text.lastIndexOf("【问题说明】");
        if (lastExplain >= 0) {
            String tail = text.substring(lastExplain + "【问题说明】".length()).trim();
            if (StringUtils.hasText(tail)) {
                return tail;
            }
        }
        return text;
    }

    private String resolveReference(
            com.alibaba.fastjson2.JSONObject issueObj,
            com.alibaba.fastjson2.JSONArray citations,
            com.alibaba.fastjson2.JSONObject firstCitation,
            Long targetFileId
    ) {
        String reference = issueObj == null ? null : issueObj.getString("reference");
        String fromCitations = resolveKnowledgeFromCitations(citations);
        if (StringUtils.hasText(fromCitations)) {
            return fromCitations;
        }
        String fallbackKnowledgeName = resolveKnowledgeFileNameFallback(citations);
        if (StringUtils.hasText(fallbackKnowledgeName)) {
            return buildSourceRef("knowledge", null, fallbackKnowledgeName);
        }
        if (isSpecificReference(reference) && reference.trim().toLowerCase().startsWith("source://knowledge/")) {
            return reference.trim();
        }
        return "unknown";
    }

    private Integer resolveBestPageNumber(com.alibaba.fastjson2.JSONArray citations, Long targetFileId) {
        if (citations == null || citations.isEmpty()) {
            return null;
        }
        Integer firstPositive = null;
        Integer firstNonOne = null;
        for (int i = 0; i < citations.size(); i++) {
            com.alibaba.fastjson2.JSONObject citation = citations.getJSONObject(i);
            if (citation == null) {
                continue;
            }
            com.alibaba.fastjson2.JSONObject meta = citation.getJSONObject("meta");
            if (meta == null) {
                continue;
            }
            if (!isCurrentTenderCitation(meta, targetFileId)) {
                continue;
            }
            Integer page = parseCitationPage(meta);
            if (page == null || page <= 0) {
                continue;
            }
            if (firstPositive == null) {
                firstPositive = page;
            }
            if (page > 1 && firstNonOne == null) {
                firstNonOne = page;
            }
        }
        return firstNonOne != null ? firstNonOne : firstPositive;
    }

    private Integer parseCitationPage(com.alibaba.fastjson2.JSONObject meta) {
        if (meta == null) {
            return null;
        }
        Integer pageNumber = meta.getInteger("pageNumber");
        if (pageNumber != null && pageNumber > 0) {
            return pageNumber;
        }
        Integer pageStart = meta.getInteger("page_start");
        if (pageStart != null && pageStart > 0) {
            return pageStart;
        }
        Integer page = meta.getInteger("page");
        if (page != null && page > 0) {
            return page;
        }
        Long pageNumLong = parseLong(firstNonBlank(
                valueToString(meta.get("page_start")),
                valueToString(meta.get("pageNumber")),
                valueToString(meta.get("page"))
        ));
        if (pageNumLong != null && pageNumLong > 0) {
            return pageNumLong.intValue();
        }
        return null;
    }

    private String resolveBestSectionName(
            com.alibaba.fastjson2.JSONArray citations,
            com.alibaba.fastjson2.JSONObject firstCitation,
            Long targetFileId
    ) {
        if (citations != null && !citations.isEmpty()) {
            for (int i = 0; i < citations.size(); i++) {
                com.alibaba.fastjson2.JSONObject citation = citations.getJSONObject(i);
                if (citation == null) {
                    continue;
                }
                com.alibaba.fastjson2.JSONObject meta = citation.getJSONObject("meta");
                if (meta == null) {
                    continue;
                }
                if (!isCurrentTenderCitation(meta, targetFileId)) {
                    continue;
                }
                String sectionName = firstNonBlank(
                        meta.getString("sectionName"),
                        meta.getString("section_name"),
                        meta.getString("title_path")
                );
                if (StringUtils.hasText(sectionName)) {
                    return sectionName;
                }
            }
        }
        if (firstCitation != null) {
            com.alibaba.fastjson2.JSONObject firstMeta = firstCitation.getJSONObject("meta");
            if (firstMeta != null && isCurrentTenderCitation(firstMeta, targetFileId)) {
                return firstNonBlank(
                        firstMeta.getString("sectionName"),
                        firstMeta.getString("section_name"),
                        firstMeta.getString("title_path")
                );
            }
        }
        return null;
    }

    private String resolveKnowledgeFromCitations(com.alibaba.fastjson2.JSONArray citations) {
        if (citations == null || citations.isEmpty()) {
            return null;
        }
        for (int i = 0; i < citations.size(); i++) {
            com.alibaba.fastjson2.JSONObject citation = citations.getJSONObject(i);
            if (citation == null) {
                continue;
            }
            com.alibaba.fastjson2.JSONObject meta = citation.getJSONObject("meta");
            if (meta == null) {
                continue;
            }
            String sourceType = firstNonBlank(meta.getString("source_type"), meta.getString("sourceType"));
            if (!"knowledge".equalsIgnoreCase(sourceType)) {
                continue;
            }
            Long fileId = parseLong(firstNonBlank(
                    valueToString(meta.get("file_id")),
                    valueToString(meta.get("fileId")),
                    valueToString(meta.get("document_id"))
            ));
            String fileName = firstNonBlank(
                    meta.getString("file_name"),
                    meta.getString("fileName"),
                    meta.getString("filename"),
                    meta.getString("documentName"),
                    meta.getString("sourceFile"),
                    meta.getString("source_file")
            );
            if (StringUtils.hasText(fileName) && !isLikelySourceFileName(fileName)) {
                fileName = null;
            }
            if (!StringUtils.hasText(fileName) && fileId != null) {
                fileName = resolveFileNameById(fileId, sourceType);
            }
            if (!StringUtils.hasText(fileName) && "knowledge".equalsIgnoreCase(sourceType)) {
                fileName = "知识库文档";
            }
            if (!StringUtils.hasText(fileName)) {
                fileName = "知识库文档";
            }
            String built = buildSourceRef(sourceType, fileId, fileName);
            return built;
        }
        return null;
    }

    private String resolveKnowledgeFileNameFallback(com.alibaba.fastjson2.JSONArray citations) {
        if (citations == null || citations.isEmpty()) {
            return null;
        }
        for (int i = 0; i < citations.size(); i++) {
            com.alibaba.fastjson2.JSONObject citation = citations.getJSONObject(i);
            if (citation == null) {
                continue;
            }
            com.alibaba.fastjson2.JSONObject meta = citation.getJSONObject("meta");
            if (meta == null) {
                continue;
            }
            String sourceType = firstNonBlank(meta.getString("source_type"), meta.getString("sourceType"));
            if (!"knowledge".equalsIgnoreCase(sourceType)) {
                continue;
            }
            String fileName = firstNonBlank(
                    meta.getString("file_name"),
                    meta.getString("fileName"),
                    meta.getString("filename"),
                    meta.getString("documentName"),
                    meta.getString("sourceFile"),
                    meta.getString("source_file"),
                    meta.getString("title_path")
            );
            if (!StringUtils.hasText(fileName)) {
                continue;
            }
            if (!isLikelySourceFileName(fileName)) {
                continue;
            }
            String normalized = fileName.replace("\\", "/");
            int lastSlash = normalized.lastIndexOf('/');
            return lastSlash >= 0 ? normalized.substring(lastSlash + 1) : normalized;
        }
        return null;
    }

    private boolean isCurrentTenderCitation(com.alibaba.fastjson2.JSONObject meta, Long targetFileId) {
        if (meta == null) {
            return false;
        }
        String sourceType = firstNonBlank(meta.getString("source_type"), meta.getString("sourceType"));
        if (!"tender".equalsIgnoreCase(sourceType)) {
            return false;
        }
        if (targetFileId == null) {
            return true;
        }
        Long fileId = parseLong(firstNonBlank(
                valueToString(meta.get("file_id")),
                valueToString(meta.get("fileId")),
                valueToString(meta.get("document_id"))
        ));
        return fileId != null && fileId.equals(targetFileId);
    }

    private boolean hasKnowledgeCitation(com.alibaba.fastjson2.JSONArray citations) {
        if (citations == null || citations.isEmpty()) {
            return false;
        }
        for (int i = 0; i < citations.size(); i++) {
            com.alibaba.fastjson2.JSONObject citation = citations.getJSONObject(i);
            if (citation == null) {
                continue;
            }
            com.alibaba.fastjson2.JSONObject meta = citation.getJSONObject("meta");
            if (meta == null) {
                continue;
            }
            String sourceType = firstNonBlank(meta.getString("source_type"), meta.getString("sourceType"));
            if ("knowledge".equalsIgnoreCase(sourceType)) {
                return true;
            }
            if (StringUtils.hasText(meta.getString("title_path"))) {
                return true;
            }
        }
        return false;
    }

    private boolean isTenderLikeReference(String reference) {
        if (!StringUtils.hasText(reference)) {
            return false;
        }
        String normalized = reference.trim().toLowerCase();
        if (normalized.startsWith("source://tender/")) {
            return true;
        }
        return normalized.endsWith(".pdf")
                || normalized.endsWith(".doc")
                || normalized.endsWith(".docx")
                || normalized.contains("招标文件")
                || normalized.contains("投标文件");
    }

    private String resolveFileNameById(Long fileId, String sourceType) {
        if (fileId == null) {
            return null;
        }
        if ("knowledge".equalsIgnoreCase(sourceType)) {
            KnowledgeFile file = knowledgeFileMapper.selectById(fileId);
            return file == null ? null : file.getFileName();
        }
        if ("tender".equalsIgnoreCase(sourceType)) {
            Tender tender = tenderMapper.selectById(fileId);
            return tender == null ? null : tender.getFileName();
        }
        KnowledgeFile file = knowledgeFileMapper.selectById(fileId);
        if (file != null && StringUtils.hasText(file.getFileName())) {
            return file.getFileName();
        }
        Tender tender = tenderMapper.selectById(fileId);
        return tender == null ? null : tender.getFileName();
    }

    private String buildSourceRef(String sourceType, Long fileId, String fileName) {
        String normalizedType = StringUtils.hasText(sourceType) ? sourceType.toLowerCase() : "unknown";
        String encodedName = java.net.URLEncoder.encode(fileName, java.nio.charset.StandardCharsets.UTF_8);
        if (fileId == null) {
            return "source://" + normalizedType + "/0/" + encodedName;
        }
        return "source://" + normalizedType + "/" + fileId + "/" + encodedName;
    }

    private String valueToString(Object value) {
        return value == null ? null : String.valueOf(value);
    }

    private Long parseLong(String value) {
        if (!StringUtils.hasText(value)) {
            return null;
        }
        try {
            return Long.parseLong(value.trim());
        } catch (Exception e) {
            return null;
        }
    }

    private boolean isSpecificReference(String reference) {
        if (!StringUtils.hasText(reference)) {
            return false;
        }
        String normalized = reference.trim().toLowerCase();
        if (normalized.equals("documents")
                || normalized.equals("document")
                || normalized.equals("docs")
                || normalized.equals("rag")
                || normalized.equals("knowledge_base")) {
            return false;
        }
        return true;
    }

    private String firstNonBlank(String... values) {
        if (values == null || values.length == 0) {
            return null;
        }
        for (String value : values) {
            if (StringUtils.hasText(value)) {
                return value;
            }
        }
        return null;
    }

    private String buildAnchorKeyForPageNormalization(
            com.alibaba.fastjson2.JSONObject issueObj,
            AuditIssueEntity issue
    ) {
        String fromContext = compactForAnchor(StringUtils.hasText(issue.getContext()) ? issue.getContext() : null);
        if (fromContext.length() >= 12) {
            return fromContext.substring(0, Math.min(120, fromContext.length()));
        }
        String rationale = issueObj == null ? null : issueObj.getString("rationale");
        String fromQuote = compactForAnchor(extractAnchorQuote(rationale));
        if (fromQuote.length() >= 12) {
            return fromQuote.substring(0, Math.min(120, fromQuote.length()));
        }
        String fallback = compactForAnchor(firstNonBlank(issue.getDescription(), rationale));
        if (fallback.length() >= 12) {
            return fallback.substring(0, Math.min(120, fallback.length()));
        }
        return "";
    }

    private String extractAnchorQuote(String text) {
        if (!StringUtils.hasText(text)) {
            return null;
        }
        int pos = text.indexOf("【问题定位】");
        if (pos < 0) {
            return null;
        }
        int q1 = text.indexOf("\"", pos);
        int q2 = q1 >= 0 ? text.indexOf("\"", q1 + 1) : -1;
        if (q1 >= 0 && q2 > q1) {
            return text.substring(q1 + 1, q2);
        }
        int c1 = text.indexOf("“", pos);
        int c2 = c1 >= 0 ? text.indexOf("”", c1 + 1) : -1;
        if (c1 >= 0 && c2 > c1) {
            return text.substring(c1 + 1, c2);
        }
        return null;
    }

    private String compactForAnchor(String value) {
        if (!StringUtils.hasText(value)) {
            return "";
        }
        return value
                .replaceAll("[\\s，,。；;：:!?！？、（）()\\[\\]【】\"“”'‘’]", "")
                .trim()
                .toLowerCase();
    }

    private com.ithsd.smart_tender.pojo.vo.AuditCompleteVO toCompleteVO(AuditTaskEntity task, List<AuditIssueEntity> issues) {
        com.ithsd.smart_tender.pojo.vo.AuditCompleteVO completeVO = new com.ithsd.smart_tender.pojo.vo.AuditCompleteVO();
        completeVO.setTaskId(task.getTaskId());
        completeVO.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        completeVO.setAuditResult(task.getAuditResult());
        completeVO.setIssueCount(defaultIfNull((long) task.getIssueCount()).intValue());
        completeVO.setFailedStages(task.getFailedStages() == null ? List.of() : task.getFailedStages());
        completeVO.setSummary(buildSummary(issues));
        return completeVO;
    }

    private SummaryVO buildSummary(List<AuditIssueEntity> issues) {
        SummaryVO summaryVO = new SummaryVO();
        int critical = 0;
        int warning = 0;
        int info = 0;
        for (AuditIssueEntity issue : issues) {
            if ("critical".equals(issue.getSeverity())) {
                critical++;
            } else if ("warning".equals(issue.getSeverity())) {
                warning++;
            } else if ("info".equals(issue.getSeverity())) {
                info++;
            }
        }
        summaryVO.setTotalIssues(issues.size());
        summaryVO.setCritical(critical);
        summaryVO.setWarning(warning);
        summaryVO.setInfo(info);
        return summaryVO;
    }

    private AuditTaskEntity loadTask(String taskId) {
        return auditTaskRepository.findByTaskId(taskId)
                .orElseThrow(() -> new BizException(404, "任务不存在"));
    }

    private String buildTaskId() {
        // taskId 格式：task_{毫秒时间戳}_{8位随机串}
        // 目标：可读性 + 低碰撞概率；并且与数据库自增 id 解耦，方便对外暴露与日志追踪
        String random = UUID.randomUUID().toString().replace("-", "").substring(0, 8);
        return "task_" + System.currentTimeMillis() + "_" + random;
    }

    private IssueVO toIssueVO(AuditIssueEntity entity) {
        IssueVO issueVO = new IssueVO();
        issueVO.setIssueNo(entity.getIssueNo());
        issueVO.setSeverity(entity.getSeverity());
        issueVO.setCategory(entity.getCategory());
        issueVO.setDimension(entity.getCategory());
        issueVO.setDescription(entity.getDescription());
        IssueVO.LocationVO locationVO = new IssueVO.LocationVO();
        locationVO.setPageNumber(entity.getPageNumber());
        locationVO.setSectionName(entity.getSectionName());
        locationVO.setContext(entity.getContext());
        issueVO.setLocation(locationVO);
        issueVO.setSuggestion(entity.getSuggestion());
        issueVO.setReference(entity.getReference());
        String anchorQuote = buildAnchorQuote(entity.getContext(), entity.getDescription());
        issueVO.setAnchorQuote(anchorQuote);
        issueVO.setAnchorPage(entity.getPageNumber());
        issueVO.setAnchorSection(entity.getSectionName());
        issueVO.setAnchorTokens(buildAnchorTokens(anchorQuote));
        issueVO.setAnchorCharsRange(buildAnchorCharsRange(entity.getContext(), anchorQuote));
        return issueVO;
    }

    private String buildAnchorQuote(String context, String description) {
        String base = firstNonBlank(
                normalizeAnchorText(context),
                extractAnchorFromDescription(description)
        );
        if (!StringUtils.hasText(base)) {
            return null;
        }
        String value = base.replaceAll("\\s+", " ").trim();
        if (value.length() > 80) {
            value = value.substring(0, 80).trim();
        }
        return value;
    }

    private boolean isLikelySourceFileName(String fileName) {
        if (!StringUtils.hasText(fileName)) {
            return false;
        }
        String normalized = fileName.trim().toLowerCase();
        if (normalized.endsWith(".pdf")
                || normalized.endsWith(".doc")
                || normalized.endsWith(".docx")
                || normalized.endsWith(".xls")
                || normalized.endsWith(".xlsx")
                || normalized.endsWith(".txt")
                || normalized.endsWith(".md")) {
            return true;
        }
        return normalized.contains("招标")
                || normalized.contains("投标")
                || normalized.contains("合同")
                || normalized.contains("文件")
                || normalized.contains("采购");
    }

    private String normalizeAnchorText(String value) {
        if (!StringUtils.hasText(value)) {
            return null;
        }
        String text = value.replaceAll("[\\r\\n\\t]+", " ").trim();
        if (!StringUtils.hasText(text)) {
            return null;
        }
        return text;
    }

    private String extractAnchorFromDescription(String description) {
        if (!StringUtils.hasText(description)) {
            return null;
        }
        String text = description;
        int marker = text.indexOf("【问题定位】");
        if (marker < 0) {
            return null;
        }
        int quoteStart = text.indexOf('\"', marker);
        if (quoteStart < 0) {
            quoteStart = text.indexOf('“', marker);
        }
        if (quoteStart < 0) {
            return null;
        }
        int quoteEnd = text.indexOf('\"', quoteStart + 1);
        if (quoteEnd < 0) {
            quoteEnd = text.indexOf('”', quoteStart + 1);
        }
        if (quoteEnd <= quoteStart) {
            return null;
        }
        return normalizeAnchorText(text.substring(quoteStart + 1, quoteEnd));
    }

    private List<String> buildAnchorTokens(String anchorQuote) {
        if (!StringUtils.hasText(anchorQuote)) {
            return List.of();
        }
        String normalized = anchorQuote
                .replaceAll("[\\r\\n\\t]+", " ")
                .replaceAll("[，,。；;：:!?！？、（）()\\[\\]【】\"“”'‘’]", " ")
                .replaceAll("\\s+", " ")
                .trim();
        if (!StringUtils.hasText(normalized)) {
            return List.of();
        }
        List<String> raw = Arrays.stream(normalized.split(" "))
                .map(String::trim)
                .filter(StringUtils::hasText)
                .filter(token -> token.length() >= 4 && token.length() <= 24)
                .toList();
        List<String> prioritized = new ArrayList<>();
        for (String token : raw) {
            if (token.matches(".*\\d.*")) {
                prioritized.add(token);
            }
        }
        for (String token : raw) {
            if (!prioritized.contains(token)) {
                prioritized.add(token);
            }
            if (prioritized.size() >= 5) {
                break;
            }
        }
        return prioritized.stream().limit(5).toList();
    }

    private List<Integer> buildAnchorCharsRange(String context, String anchorQuote) {
        if (!StringUtils.hasText(anchorQuote)) {
            return List.of();
        }
        String rawContext = StringUtils.hasText(context) ? context : anchorQuote;
        String normalizedContext = rawContext.replaceAll("\\s+", " ").trim();
        String normalizedQuote = anchorQuote.replaceAll("\\s+", " ").trim();
        if (!StringUtils.hasText(normalizedContext) || !StringUtils.hasText(normalizedQuote)) {
            return List.of();
        }
        int start = normalizedContext.indexOf(normalizedQuote);
        if (start < 0) {
            return List.of(0, normalizedQuote.length());
        }
        return List.of(start, start + normalizedQuote.length());
    }

    private String resolveAuditResult(AuditTaskEntity task, Long totalIssues) {
        // auditResult 如果已由引擎写入（pass/revise），优先返回
        if (StringUtils.hasText(task.getAuditResult())) {
            return task.getAuditResult();
        }
        // 否则做兜底推断：无问题且无失败阶段 => pass；否则 revise
        boolean hasFailedStages = task.getFailedStages() != null && !task.getFailedStages().isEmpty();
        if (totalIssues == 0 && !hasFailedStages) {
            return "pass";
        }
        return "revise";
    }

    private Long defaultIfNull(Long value) {
        if (value == null) {
            return 0L;
        }
        return value;
    }
}
