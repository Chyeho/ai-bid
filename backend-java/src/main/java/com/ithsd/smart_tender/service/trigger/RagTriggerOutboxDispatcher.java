package com.ithsd.smart_tender.service.trigger;

import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;
import com.ithsd.smart_tender.repository.RagTriggerOutboxRepository;
import io.micrometer.core.instrument.Counter;
import io.micrometer.core.instrument.Gauge;
import io.micrometer.core.instrument.MeterRegistry;
import io.micrometer.core.instrument.Timer;
import io.micrometer.core.instrument.simple.SimpleMeterRegistry;
import org.springframework.context.annotation.Lazy;
import com.ithsd.smart_tender.service.AuditTaskService;
import org.springframework.scheduling.annotation.Scheduled;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.List;
import java.util.Set;
import java.util.concurrent.atomic.AtomicLong;

@Component
public class RagTriggerOutboxDispatcher {
    private static final Set<String> PENDING_STATUSES = Set.of(
            RagTriggerOutboxService.STATUS_NEW,
            RagTriggerOutboxService.STATUS_RETRYING
    );
    private final RagTriggerOutboxRepository outboxRepository;
    private final RagTriggerHttpClient httpClient;
    private final RagTriggerProperties properties;
    private final AuditTaskService auditTaskService;
    private final Counter successCounter;
    private final Counter failCounter;
    private final Timer latencyTimer;
    private final AtomicLong backlogSize = new AtomicLong(0);
    private final AtomicLong dlqSize = new AtomicLong(0);

    public RagTriggerOutboxDispatcher(
            RagTriggerOutboxRepository outboxRepository,
            RagTriggerHttpClient httpClient,
            RagTriggerProperties properties,
            @Lazy AuditTaskService auditTaskService,
            MeterRegistry meterRegistry
    ) {
        this.outboxRepository = outboxRepository;
        this.httpClient = httpClient;
        this.properties = properties;
        this.auditTaskService = auditTaskService;
        MeterRegistry registry = meterRegistry == null ? new SimpleMeterRegistry() : meterRegistry;
        this.successCounter = registry.counter("rag_trigger_success_total");
        this.failCounter = registry.counter("rag_trigger_fail_total");
        this.latencyTimer = registry.timer("rag_trigger_latency_ms");
        Gauge.builder("outbox_backlog_size", backlogSize, AtomicLong::get).register(registry);
        Gauge.builder("dlq_size", dlqSize, AtomicLong::get).register(registry);
    }

    @Scheduled(fixedDelayString = "${rag.trigger.poll-delay-ms:2000}")
    public void dispatch() {
        if (!properties.isEnabled()) {
            refreshGauge();
            return;
        }
        List<RagTriggerOutboxEntity> list = outboxRepository.findTop50ByStatusInAndNextRetryAtLessThanEqualOrderByIdAsc(PENDING_STATUSES, LocalDateTime.now());
        for (RagTriggerOutboxEntity entity : list) {
            dispatchOne(entity);
        }
        refreshGauge();
    }

    void dispatchOne(RagTriggerOutboxEntity entity) {
        boolean isAuditTask = StringUtils.hasText(entity.getJobId()) && !entity.getJobId().startsWith("file-");
        if (isAuditTask) {
            auditTaskService.markTaskProcessing(entity.getJobId());
        }
        entity.setStatus(RagTriggerOutboxService.STATUS_SENDING);
        entity.setUpdatedAt(LocalDateTime.now());
        outboxRepository.save(entity);
        Timer.Sample sample = Timer.start();
        RagTriggerHttpClient.TriggerHttpResult result = httpClient.postTrigger(entity);
        sample.stop(latencyTimer);
        entity.setLastStatusCode(result.statusCode());
        entity.setResponseBody(result.body());
        entity.setLastErrorMsg(result.errorMessage());
        entity.setUpdatedAt(LocalDateTime.now());
        if (result.success()) {
            entity.setStatus(RagTriggerOutboxService.STATUS_SENT);
            entity.setSentAt(LocalDateTime.now());
            outboxRepository.save(entity);
            successCounter.increment();
            
            // 针对审核任务，解析结果并入库
            if (isAuditTask) {
                auditTaskService.processAuditResult(entity.getJobId(), result.body());
            }
            return;
        }
        boolean canRetry = result.retryable() && entity.getRetryCount() < entity.getMaxRetry();
        if (canRetry) {
            int retryCount = entity.getRetryCount() + 1;
            entity.setRetryCount(retryCount);
            entity.setStatus(RagTriggerOutboxService.STATUS_RETRYING);
            entity.setNextRetryAt(LocalDateTime.now().plusNanos(backoffMillis(retryCount) * 1_000_000L));
            outboxRepository.save(entity);
        } else {
            entity.setStatus(RagTriggerOutboxService.STATUS_DLQ);
            entity.setNextRetryAt(LocalDateTime.now());
            outboxRepository.save(entity);
            failCounter.increment();
            if (isAuditTask) {
                auditTaskService.markTaskFailed(entity.getJobId(), result.errorMessage());
            }
        }
    }

    private long backoffMillis(int retryCount) {
        long base = properties.getInitialBackoffMs();
        return base * (1L << Math.max(0, retryCount - 1));
    }

    private void refreshGauge() {
        backlogSize.set(outboxRepository.countByStatusIn(PENDING_STATUSES));
        dlqSize.set(outboxRepository.countByStatus(RagTriggerOutboxService.STATUS_DLQ));
    }
}
