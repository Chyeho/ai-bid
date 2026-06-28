package com.ithsd.smart_tender.pojo.entity;

import com.baomidou.mybatisplus.annotation.IdType;
import com.baomidou.mybatisplus.annotation.TableId;
import com.baomidou.mybatisplus.annotation.TableName;
import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;
import java.time.LocalDateTime;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
@TableName("audit_task")
public class AuditTask implements Serializable {
    @TableId(value = "id", type = IdType.AUTO)
    private Long id;

    private String taskId;

    private Long bidId;

    private Integer taskStatus;

    private String auditResult;

    private Integer issueCount;

    private Integer criticalCount;

    private Integer warningCount;

    private Integer infoCount;

    private LocalDateTime startTime;

    private LocalDateTime endTime;

    private Long auditUserId;

    private LocalDateTime createTime;
}
