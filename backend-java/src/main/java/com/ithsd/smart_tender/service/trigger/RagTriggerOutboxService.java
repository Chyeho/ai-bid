package com.ithsd.smart_tender.service.trigger;

import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;
import com.ithsd.smart_tender.repository.RagTriggerOutboxRepository;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.Optional;
import java.util.UUID;

@Service
public class RagTriggerOutboxService {
    public static final String STATUS_NEW = "NEW";
    public static final String STATUS_SENDING = "SENDING";
    public static final String STATUS_SENT = "SENT";
    public static final String STATUS_RETRYING = "RETRYING";
    public static final String STATUS_DLQ = "DLQ";
    private final RagTriggerOutboxRepository outboxRepository;
    private final RagTriggerProperties properties;

    public RagTriggerOutboxService(RagTriggerOutboxRepository outboxRepository, RagTriggerProperties properties) {
        this.outboxRepository = outboxRepository;
        this.properties = properties;
    }

    @Transactional
    public RagTriggerOutboxEntity enqueue(Long fileId, Integer chunkCount, String strategyVersion, String payloadHash, String jobId) {
        String safeJobId = StringUtils.hasText(jobId) ? jobId : "file-" + fileId;
        String idempotencyKey = safeJobId + ":" + payloadHash;
        Optional<RagTriggerOutboxEntity> existed = outboxRepository.findByIdempotencyKey(idempotencyKey);
        if (existed.isPresent()) {
            return existed.get();
        }
        LocalDateTime now = LocalDateTime.now();
        RagTriggerOutboxEntity entity = new RagTriggerOutboxEntity();
        entity.setRequestId(UUID.randomUUID().toString().replace("-", ""));
        entity.setJobId(safeJobId);
        entity.setFileId(fileId);
        entity.setChunkCount(chunkCount);
        entity.setStrategyVersion(strategyVersion);
        entity.setPayloadHash(payloadHash);
        entity.setIdempotencyKey(idempotencyKey);
        entity.setEndpoint(properties.getEndpoint());
        entity.setStatus(STATUS_NEW);
        entity.setRetryCount(0);
        entity.setMaxRetry(properties.getMaxRetry());
        entity.setNextRetryAt(now);
        entity.setCreatedAt(now);
        entity.setUpdatedAt(now);
        return outboxRepository.save(entity);
    }
}
