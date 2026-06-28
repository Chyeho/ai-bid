package com.ithsd.smart_tender.pojo.dto;

import lombok.Data;
import java.io.Serializable;

@Data
public class ChatCommitRequestDTO implements Serializable {
    private Long projectId;
    private Long bidId;
    private Boolean useLatest;
    private String userContent;
    private String aiContent;
    private Boolean normalizeBeforeSave;
}
