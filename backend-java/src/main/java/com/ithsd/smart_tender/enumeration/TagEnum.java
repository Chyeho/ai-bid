package com.ithsd.smart_tender.enumeration;

import lombok.Getter;

import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;

/**
 * 二级标签枚举
 */
@Getter
public enum TagEnum {
    
    PROCUREMENT("采购类", "procurement"),
    ENGINEERING("工程类", "engineering"),
    GENERAL("通用", "general");
    
    private final String displayName;
    private final String code;
    
    TagEnum(String displayName, String code) {
        this.displayName = displayName;
        this.code = code;
    }
    
    /**
     * 根据code获取枚举
     */
    public static TagEnum fromCode(String code) {
        for (TagEnum tag : TagEnum.values()) {
            if (tag.getCode().equals(code)) {
                return tag;
            }
        }
        throw new IllegalArgumentException("未知的二级标签code: " + code);
    }
    
    /**
     * 根据显示名称获取枚举
     */
    public static TagEnum fromDisplayName(String displayName) {
        for (TagEnum tag : TagEnum.values()) {
            if (tag.getDisplayName().equals(displayName)) {
                return tag;
            }
        }
        throw new IllegalArgumentException("未知的二级标签显示名称: " + displayName);
    }
    
    /**
     * 将逗号分隔的标签字符串转换为枚举列表
     */
    public static List<TagEnum> fromTagString(String tagString) {
        if (tagString == null || tagString.trim().isEmpty()) {
            return Arrays.asList();
        }
        return Arrays.stream(tagString.split(","))
                .map(String::trim)
                .filter(s -> !s.isEmpty())
                .map(TagEnum::fromDisplayName)
                .collect(Collectors.toList());
    }
    
    /**
     * 将枚举列表转换为逗号分隔的字符串
     */
    public static String toTagString(List<TagEnum> tagEnums) {
        if (tagEnums == null || tagEnums.isEmpty()) {
            return "";
        }
        return tagEnums.stream()
                .map(TagEnum::getDisplayName)
                .collect(Collectors.joining(","));
    }
}
