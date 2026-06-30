package com.ithsd.smart_tender.model.vo;

import lombok.Builder;
import lombok.Data;
import lombok.AllArgsConstructor;
import lombok.NoArgsConstructor;

import java.util.List;

/**
 * 标准库文件列表项VO
 * 前端Table组件每行展示的数据
 */
@Data
@AllArgsConstructor
@NoArgsConstructor
@Builder
public class KnowledgeFileVO {
   

    /**
     * 文件ID（前端操作按钮关联）
     */
    private Long id;

    /**
     * 文件名
     */
    private String fileName;

    /**
     * 文件大小（格式化：如1.2MB）
     */
    private String fileSize;

    /**
     * 文件类型名称（如PDF文件、Word文件）
     */
    private String fileType;

    /**
     * 分类名称（如制度、价格）
     */
    private String category;

    /**
     * 标签列表（拆分后的标签，如["采购类","工程类"]）
     */
    private String tag;

    /**
     * 用途描述
     */
    private String description;

    /**
     * 适用范围（如：procurement, engineering, general）
     */
    private String applicableScope;

    /**
     * 状态名称（如启用、停用、已删除）
     */
    private String status;

    /**
     * 版本号
     */
    private Integer version;

    /**
     * 上传用户ID（可选）
     */
    private Long uploadUserId;

    /**
     * 上传用户姓名
     */
    private String uploadUserName;

    /**
     * 上传时间（格式化：yyyy-MM-dd HH:mm:ss）
     */
    private String uploadTime;

    /**
     * 更新时间（格式化：yyyy-MM-dd HH:mm:ss）
     */
    private String updateTime;
}