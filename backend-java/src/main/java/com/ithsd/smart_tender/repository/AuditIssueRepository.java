package com.ithsd.smart_tender.repository;

import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import org.springframework.data.domain.Page;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.List;

public interface AuditIssueRepository extends JpaRepository<AuditIssueEntity, Long> {
    List<AuditIssueEntity> findByAuditIdOrderByIssueNoAsc(Long auditId);

    Page<AuditIssueEntity> findByAuditIdOrderByIssueNoAsc(Long auditId, Pageable pageable);

    Page<AuditIssueEntity> findByAuditIdAndIssueNoGreaterThanOrderByIssueNoAsc(Long auditId, String issueNo, Pageable pageable);

    Long countByAuditId(Long auditId);

    Long countByAuditIdAndSeverity(Long auditId, String severity);
}
