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
@TableName("audit_issue")
public class AuditIssue implements Serializable {
    @TableId(value = "id", type = IdType.AUTO)
    private Long id;

    private Long auditId;

    private String issueNo;

    private String severity;

    private String category;

    private String description;

    private String suggestion;

    private Integer pageNumber;

    private String sectionName;

    private String context;

    private String reference;

    private LocalDateTime createTime;
}
