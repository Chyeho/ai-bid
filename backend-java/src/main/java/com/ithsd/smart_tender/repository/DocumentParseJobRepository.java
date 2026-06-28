package com.ithsd.smart_tender.repository;

import com.ithsd.smart_tender.pojo.entity.DocumentParseJobEntity;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.Collection;
import java.util.List;
import java.util.Optional;

public interface DocumentParseJobRepository extends JpaRepository<DocumentParseJobEntity, Long> {
    Optional<DocumentParseJobEntity> findByJobId(String jobId);

    Optional<DocumentParseJobEntity> findTopByRequestIdOrderByCreatedAtDesc(String requestId);

    List<DocumentParseJobEntity> findByFileIdAndStrategyVersionAndStatusIn(Long fileId, String strategyVersion, Collection<String> statuses);
}
