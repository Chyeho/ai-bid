package com.ithsd.smart_tender.model.dto;

import lombok.Data;
import java.io.Serializable;

@Data
public class ProjectDTO implements Serializable {
    private Long id; // 修改时使用
    private String projectName;
    private String supplierName;
}
