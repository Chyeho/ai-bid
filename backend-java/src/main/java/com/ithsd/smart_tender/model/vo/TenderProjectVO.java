package com.ithsd.smart_tender.model.vo;

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
public class TenderProjectVO implements Serializable {
    private Long projectId;          // 项目ID
    private String projectName;      // 项目名称（版本1.0的bid_name）
    private LocalDateTime createTime;// 创建时间（版本1.0的上传时间）
    private String creatorName;      // 创建人真实姓名（版本1.0的上传用户）
    private Integer latestVersion;   // 最新版本号
    private String supplierName;     // 供应商名称（版本1.0）
    private String fileCategory;     // 标书类型（版本1.0）
    private String auditorName;      // 审核人姓名（最新版本的审核人）
}
