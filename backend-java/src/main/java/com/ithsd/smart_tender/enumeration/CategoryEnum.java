package com.ithsd.smart_tender.enumeration;

import lombok.Getter;

/**
 * 一级分类枚举
 */
@Getter
public enum CategoryEnum {
    
    REGULATION("制度文件", "regulation"),
    PRICE_STANDARD("价格标准", "price_standard"),
    SUPPLIER_LIST("供应商名录", "supplier_list"),
    CONTRACT_TEMPLATE("合同模板", "contract_template"),
    CASE_LIBRARY("案例库", "case_library"),
    OTHER("其他", "other");
    
    private final String displayName;
    private final String code;
    
    CategoryEnum(String displayName, String code) {
        this.displayName = displayName;
        this.code = code;
    }
    
    /**
     * 根据code获取枚举
     */
    public static CategoryEnum fromCode(String code) {
        for (CategoryEnum category : CategoryEnum.values()) {
            if (category.getCode().equals(code)) {
                return category;
            }
        }
        throw new IllegalArgumentException("未知的一级分类code: " + code);
    }
    
    /**
     * 根据显示名称获取枚举
     */
    public static CategoryEnum fromDisplayName(String displayName) {
        for (CategoryEnum category : CategoryEnum.values()) {
            if (category.getDisplayName().equals(displayName)) {
                return category;
            }
        }
        throw new IllegalArgumentException("未知的一级分类显示名称: " + displayName);
    }
}
