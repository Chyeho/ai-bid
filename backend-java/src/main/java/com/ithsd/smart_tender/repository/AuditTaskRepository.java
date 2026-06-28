package com.ithsd.smart_tender.repository;

import com.ithsd.smart_tender.pojo.entity.AuditTaskEntity;
import org.springframework.data.jpa.repository.JpaRepository;

import java.util.Optional;

public interface AuditTaskRepository extends JpaRepository<AuditTaskEntity, Long> {
    Optional<AuditTaskEntity> findByTaskId(String taskId);

    Long countByTaskStatus(Integer taskStatus);
}
