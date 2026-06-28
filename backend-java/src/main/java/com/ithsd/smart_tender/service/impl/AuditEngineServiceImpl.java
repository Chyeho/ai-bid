package com.ithsd.smart_tender.service.impl;

import io.micrometer.core.instrument.Counter;
import io.micrometer.core.instrument.MeterRegistry;
import io.micrometer.core.instrument.Timer;
import io.micrometer.core.instrument.simple.SimpleMeterRegistry;
import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import com.ithsd.smart_tender.pojo.entity.AuditTaskEntity;
import com.ithsd.smart_tender.pojo.enums.AuditStageEnum;
import com.ithsd.smart_tender.pojo.enums.AuditTaskStatusEnum;
import com.ithsd.smart_tender.pojo.enums.SseEventTypeEnum;
import com.ithsd.smart_tender.pojo.vo.AuditCompleteVO;
import com.ithsd.smart_tender.pojo.vo.AuditTaskStatusVO;
import com.ithsd.smart_tender.pojo.vo.IssueVO;
import com.ithsd.smart_tender.pojo.vo.SummaryVO;
import com.ithsd.smart_tender.repository.AuditIssueRepository;
import com.ithsd.smart_tender.repository.AuditTaskRepository;
import com.ithsd.smart_tender.service.AuditEngineService;
import com.ithsd.smart_tender.service.engine.AuditChainRunner;
import com.ithsd.smart_tender.service.engine.AuditContext;
import com.ithsd.smart_tender.service.engine.AuditProgressProperties;
import com.ithsd.smart_tender.service.extract.AuditExtractProperties;
import com.ithsd.smart_tender.service.extract.DocumentExtractService;
import com.ithsd.smart_tender.service.extract.ExtractedDocument;
import com.ithsd.smart_tender.service.rag.StandardRagService;
import com.ithsd.smart_tender.sse.AuditTaskEventService;
import com.ithsd.smart_tender.sse.SseHub;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Qualifier;
import org.springframework.scheduling.annotation.Async;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

@Service
public class AuditEngineServiceImpl implements AuditEngineService {
    private static final int DEFAULT_FAILED_PROGRESS = 20;
    private static final Set<String> RUNNING_TASKS = ConcurrentHashMap.newKeySet();
    private static final Logger log = LoggerFactory.getLogger(AuditEngineServiceImpl.class);
    private final AuditTaskRepository auditTaskRepository;
    private final AuditIssueRepository auditIssueRepository;
    private final SseHub sseHub;
    private final AuditChainRunner auditChainRunner;
    private final StandardRagService standardRagService;
    private final DocumentExtractService stubDocumentExtractService;
    private final DocumentExtractService placeholderDocumentExtractService;
    private final DocumentExtractService wordDocumentExtractService;
    private final AuditExtractProperties extractProperties;
    private final AuditTaskEventService eventService;
    private final AuditProgressProperties progressProperties;
    private final MeterRegistry meterRegistry;
    private final Counter concurrentSkipCounter;
    private final Counter completedCounter;
    private final Counter failedCounter;
    private final Counter issueEmitCounter;
    private final Counter persistFailureCounter;
    private final Timer taskTimer;

    public AuditEngineServiceImpl(
            AuditTaskRepository auditTaskRepository,
            AuditIssueRepository auditIssueRepository,
            SseHub sseHub,
            AuditChainRunner auditChainRunner,
            StandardRagService standardRagService,
            @Qualifier("stubDocumentExtractService") DocumentExtractService stubDocumentExtractService,
            @Qualifier("placeholderDocumentExtractService") DocumentExtractService placeholderDocumentExtractService,
            @Qualifier("wordDocumentExtractService") DocumentExtractService wordDocumentExtractService,
            AuditExtractProperties extractProperties,
            AuditTaskEventService eventService,
            AuditProgressProperties progressProperties
    ) {
        this(auditTaskRepository, auditIssueRepository, sseHub, auditChainRunner, standardRagService, stubDocumentExtractService, placeholderDocumentExtractService, wordDocumentExtractService, extractProperties, eventService, progressProperties, null);
    }

    @Autowired
    public AuditEngineServiceImpl(
            AuditTaskRepository auditTaskRepository,
            AuditIssueRepository auditIssueRepository,
            SseHub sseHub,
            AuditChainRunner auditChainRunner,
            StandardRagService standardRagService,
            @Qualifier("stubDocumentExtractService") DocumentExtractService stubDocumentExtractService,
            @Qualifier("placeholderDocumentExtractService") DocumentExtractService placeholderDocumentExtractService,
            @Qualifier("wordDocumentExtractService") DocumentExtractService wordDocumentExtractService,
            AuditExtractProperties extractProperties,
            AuditTaskEventService eventService,
            AuditProgressProperties progressProperties,
            MeterRegistry meterRegistry
    ) {
        this.auditTaskRepository = auditTaskRepository;
        this.auditIssueRepository = auditIssueRepository;
        this.sseHub = sseHub;
        this.auditChainRunner = auditChainRunner;
        this.standardRagService = standardRagService;
        this.stubDocumentExtractService = stubDocumentExtractService;
        this.placeholderDocumentExtractService = placeholderDocumentExtractService;
        this.wordDocumentExtractService = wordDocumentExtractService;
        this.extractProperties = extractProperties;
        this.eventService = eventService;
        this.progressProperties = progressProperties;
        this.meterRegistry = meterRegistry == null ? new SimpleMeterRegistry() : meterRegistry;
        this.concurrentSkipCounter = this.meterRegistry.counter("audit.task.concurrent.skip.total");
        this.completedCounter = this.meterRegistry.counter("audit.task.completed.total");
        this.failedCounter = this.meterRegistry.counter("audit.task.failed.total");
        this.issueEmitCounter = this.meterRegistry.counter("audit.task.issue.emit.total");
        this.persistFailureCounter = this.meterRegistry.counter("audit.task.persist.failure.total");
        this.taskTimer = this.meterRegistry.timer("audit.task.duration");
    }

    @Override
    @Async("auditTaskExecutor")
    @Transactional
    public void start(String taskId) {
        if (!RUNNING_TASKS.add(taskId)) {
            concurrentSkipCounter.increment();
            log.warn("audit task skipped due to concurrent start, taskId={}", taskId);
            return;
        }
        Timer.Sample sample = Timer.start(meterRegistry);
        try {
            startInternal(taskId);
        } finally {
            sample.stop(taskTimer);
            RUNNING_TASKS.remove(taskId);
        }
    }

    void startInternal(String taskId) {
        AuditTaskEntity task = auditTaskRepository.findByTaskId(taskId).orElse(null);
        if (task == null) {
            log.warn("audit task not found, taskId={}", taskId);
            concurrentSkipCounter.increment();
            sseHub.close(taskId);
            return;
        }
        if (!AuditTaskStatusEnum.PENDING.getCode().equals(task.getTaskStatus())) {
            concurrentSkipCounter.increment();
            log.info("audit task already started or finished, taskId={}, status={}", taskId, task.getTaskStatus());
            sseHub.close(taskId);
            return;
        }
        LocalDateTime now = LocalDateTime.now();
        try {
            task.setTaskStatus(AuditTaskStatusEnum.PROCESSING.getCode());
            task.setStage(AuditStageEnum.DOC_EXTRACT.name());
            task.setProgress(progressProperties.getDocExtract());
            task.setStartTime(now);
            task.setFailedStages(new ArrayList<>());
            task.setUpdatedAt(now);
            if (!persistTask(task)) {
                return;
            }
            emitSafe(taskId, SseEventTypeEnum.PROGRESS, toStatusVO(task));

            task.setStage(AuditStageEnum.RAG.name());
            task.setProgress(progressProperties.beforeChecksProgress());
            task.setUpdatedAt(LocalDateTime.now());
            if (!persistTask(task)) {
                return;
            }
            emitSafe(taskId, SseEventTypeEnum.PROGRESS, toStatusVO(task));

            AuditContext context = runEngine(task);

            task.setTaskStatus(AuditTaskStatusEnum.COMPLETED.getCode());
            task.setStage(AuditStageEnum.SUMMARY.name());
            task.setProgress(100);
            task.setIssueCount(context.getIssues().size());
            task.setFailedStages(new ArrayList<>(context.getFailedStages()));
            task.setAuditResult(task.getIssueCount() > 0 || !task.getFailedStages().isEmpty() ? "revise" : "pass");
            task.setEndTime(LocalDateTime.now());
            task.setUpdatedAt(LocalDateTime.now());
            if (persistTask(task)) {
                emitSafe(taskId, SseEventTypeEnum.COMPLETE, toCompleteVO(task, context.getIssues()));
                completedCounter.increment();
            }
        } catch (Exception ex) {
            failedCounter.increment();
            log.error("audit task failed, taskId={}, stage={}", taskId, task.getStage(), ex);
            task.setTaskStatus(AuditTaskStatusEnum.FAILED.getCode());
            task.setProgress(Math.max(defaultInt(task.getProgress()), DEFAULT_FAILED_PROGRESS));
            task.setErrorMsg(crop(ex.getMessage()));
            if (!StringUtils.hasText(task.getStage())) {
                task.setStage(AuditStageEnum.CHAIN.name());
            }
            if (task.getFailedStages() == null) {
                task.setFailedStages(new ArrayList<>());
            }
            if (!task.getFailedStages().contains(task.getStage())) {
                task.getFailedStages().add(task.getStage());
            }
            task.setEndTime(LocalDateTime.now());
            task.setUpdatedAt(LocalDateTime.now());
            if (persistTask(task)) {
                emitSafe(taskId, SseEventTypeEnum.COMPLETE, toCompleteVO(task, List.of()));
            }
        } finally {
            sseHub.close(taskId);
        }
    }

    AuditContext runEngine(AuditTaskEntity task) {
        if (task.getBidId() != null && task.getBidId() < 0) {
            throw new IllegalStateException("bidId非法");
        }
        AuditContext context = AuditContext.fromTask(task.getTaskId(), task.getBidId(), task.getEnabledChecks());
        ExtractedDocument extractedDocument = extractDocument(context);
        context.setDocumentText(extractedDocument.getContent());
        context.setDocumentSource(extractedDocument.getSource());
        if (extractedDocument.isDegraded()) {
            context.addFailedStage(AuditStageEnum.DOC_EXTRACT.name());
        }
        context.setProgress(progressProperties.beforeChecksProgress());
        context.setStage(AuditStageEnum.RAG.name());
        standardRagService.retrieveAndMerge(context);
        auditChainRunner.run(context, issue -> handleFoundIssue(task, issue));
        return context;
    }

    private ExtractedDocument extractDocument(AuditContext context) {
        String provider = extractProperties.getProvider() == null ? "stub" : extractProperties.getProvider().toLowerCase();
        try {
            if ("placeholder".equals(provider)) {
                if (!extractProperties.isPlaceholderEnabled()) {
                    throw new IllegalStateException("DOC_EXTRACT_PLACEHOLDER_DISABLED");
                }
                return placeholderDocumentExtractService.extract(context.getBidId());
            }
            if ("word".equals(provider)) {
                return wordDocumentExtractService.extract(context.getBidId());
            }
            return stubDocumentExtractService.extract(context.getBidId());
        } catch (RuntimeException ex) {
            if (extractProperties.isFallbackToStub() && !"stub".equals(provider)) {
                ExtractedDocument fallback = stubDocumentExtractService.extract(context.getBidId());
                fallback.setDegraded(true);
                fallback.setErrorCode(5604);
                fallback.setErrorMessage(ex.getMessage());
                fallback.setSource("stub_fallback");
                return fallback;
            }
            throw ex;
        }
    }

    private void handleFoundIssue(AuditTaskEntity task, AuditIssueEntity issue) {
        issue.setAuditId(task.getId());
        if (issue.getCreateTime() == null) {
            issue.setCreateTime(LocalDateTime.now());
        }
        if (!StringUtils.hasText(issue.getSeverity())) {
            issue.setSeverity("warning");
        }
        if (!StringUtils.hasText(issue.getCategory())) {
            issue.setCategory("budget");
        }
        if (!StringUtils.hasText(issue.getDescription())) {
            issue.setDescription("待补充问题描述");
        }
        auditIssueRepository.save(issue);
        task.setIssueCount(defaultInt(task.getIssueCount()) + 1);
        task.setUpdatedAt(LocalDateTime.now());
        if (persistTask(task)) {
            emitSafe(task.getTaskId(), SseEventTypeEnum.ISSUE, toIssueVO(issue));
            issueEmitCounter.increment();
        }
    }

    private AuditTaskStatusVO toStatusVO(AuditTaskEntity task) {
        AuditTaskStatusVO statusVO = new AuditTaskStatusVO();
        statusVO.setTaskId(task.getTaskId());
        statusVO.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        statusVO.setStage(task.getStage());
        statusVO.setProgress(task.getProgress());
        statusVO.setIssueCount(defaultInt(task.getIssueCount()));
        statusVO.setFailedStages(task.getFailedStages() == null ? new ArrayList<>() : task.getFailedStages());
        return statusVO;
    }

    private int defaultInt(Integer value) {
        if (value == null) {
            return 0;
        }
        return value;
    }

    private IssueVO toIssueVO(AuditIssueEntity entity) {
        IssueVO issueVO = new IssueVO();
        issueVO.setIssueNo(entity.getIssueNo());
        issueVO.setSeverity(entity.getSeverity());
        issueVO.setCategory(entity.getCategory());
        issueVO.setDescription(entity.getDescription());
        IssueVO.LocationVO locationVO = new IssueVO.LocationVO();
        locationVO.setPageNumber(entity.getPageNumber());
        locationVO.setSectionName(entity.getSectionName());
        locationVO.setContext(entity.getContext());
        issueVO.setLocation(locationVO);
        issueVO.setSuggestion(entity.getSuggestion());
        issueVO.setReference(entity.getReference());
        return issueVO;
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

    private AuditCompleteVO toCompleteVO(AuditTaskEntity task, List<AuditIssueEntity> issues) {
        AuditCompleteVO completeVO = new AuditCompleteVO();
        completeVO.setTaskId(task.getTaskId());
        completeVO.setStatus(AuditTaskStatusEnum.fromCode(task.getTaskStatus()).getValue());
        completeVO.setAuditResult(task.getAuditResult());
        completeVO.setIssueCount(defaultInt(task.getIssueCount()));
        completeVO.setFailedStages(task.getFailedStages() == null ? List.of() : task.getFailedStages());
        completeVO.setSummary(buildSummary(issues));
        return completeVO;
    }

    private String crop(String value) {
        if (!StringUtils.hasText(value)) {
            return "引擎执行失败";
        }
        if (value.length() <= 1000) {
            return value;
        }
        return value.substring(0, 1000);
    }

    private boolean persistTask(AuditTaskEntity task) {
        try {
            auditTaskRepository.save(task);
            return true;
        } catch (RuntimeException ex) {
            persistFailureCounter.increment();
            log.warn("persist audit task failed, taskId={}, stage={}", task.getTaskId(), task.getStage(), ex);
            return false;
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
}
