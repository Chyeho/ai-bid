package com.ithsd.smart_tender.repository;

import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;
import org.springframework.data.jpa.repository.JpaRepository;

import java.time.LocalDateTime;
import java.util.Collection;
import java.util.List;
import java.util.Optional;

public interface RagTriggerOutboxRepository extends JpaRepository<RagTriggerOutboxEntity, Long> {
    Optional<RagTriggerOutboxEntity> findByIdempotencyKey(String idempotencyKey);

    List<RagTriggerOutboxEntity> findTop50ByStatusInAndNextRetryAtLessThanEqualOrderByIdAsc(Collection<String> statuses, LocalDateTime nextRetryAt);

    long countByStatusIn(Collection<String> statuses);

    long countByStatus(String status);

    Optional<RagTriggerOutboxEntity> findTopByJobIdOrderByCreatedAtDesc(String jobId);
}
