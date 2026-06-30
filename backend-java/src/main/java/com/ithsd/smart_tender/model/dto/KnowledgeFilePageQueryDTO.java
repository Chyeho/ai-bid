package com.ithsd.smart_tender.model.dto;

import lombok.AllArgsConstructor;
import lombok.Data;
import lombok.NoArgsConstructor;
import lombok.Builder;

/**
 * 标准库文件分页查询 DTO
 */
@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class KnowledgeFilePageQueryDTO {

    /**
     * 当前页码（从 1 开始）
     */
    private int pageNum ;

    /**
     * 每页记录数
     */
    private int pageSize ;

    /**
     * 文件名（模糊查询）
     */
    private String fileName;

    /**
     * 一级分类（精确查询）
     */
    private String category;

    /**
     * 二级标签（模糊查询）
     */
    private String tags;

    /**
     * 适用范围
     */
    private String applicableScope;

    /**
     * 文件状态（精确查询）
     */
    private String status;

    /**
     * 上传人（模糊查询）
     */
    private String uploadUser;
}
