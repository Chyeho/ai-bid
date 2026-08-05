package com.ithsd.smart_tender.model.dto;

import lombok.AllArgsConstructor;
import lombok.Data;
import lombok.NoArgsConstructor;
import lombok.Builder;

/**
 * 标准库文件查询/更新 DTO
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class KnowledgeFileDTO {

    /**
     * 主键 ID
     */
    private Long id;

    /**
     * 文件名
     */
    private String fileName;

    /**
     * 文件类型（如：pdf、docx、xlsx 等）
     */
    private String fileType;

    /**
     * 一级分类
     */
    private String category;

    /**
     * 二级标签（多个标签用逗号分隔）
     */
    private String tags;

    /**
     * 适用范围
     */
    private String applicableScope;

    /**
     * 用途描述
     */
    private String description;

    /**
     * 文件状态
     * 0: 停用
     * 1: 启用
     * 2: 已删除
     */
    private Integer status;

    /**
     * 版本号
     */
    private Integer version;


}
