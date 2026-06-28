package com.ithsd.smart_tender.repository;

import com.ithsd.smart_tender.pojo.entity.AuditTaskEventEntity;
import org.springframework.data.domain.Pageable;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.List;

public interface AuditTaskEventRepository extends JpaRepository<AuditTaskEventEntity, Long> {
    List<AuditTaskEventEntity> findByTaskIdAndIdGreaterThanOrderByIdAsc(String taskId, Long id, Pageable pageable);
}
