package com.ithsd.smart_tender.pojo.dto;

import lombok.Data;
import java.io.Serializable;

@Data
public class ChatRequestDTO implements Serializable {
    private Long projectId;
    private Long bidId;
    private String content;
    private String mode;
    private Boolean saveToKnowledgeBase; // 是否保存到知识库
    private Boolean normalizeBeforeSave; // 是否需要规范化
}
