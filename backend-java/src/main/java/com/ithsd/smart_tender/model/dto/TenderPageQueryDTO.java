package com.ithsd.smart_tender.model.dto;

import lombok.Data;
import java.io.Serializable;
import java.time.LocalDate;
import java.time.LocalDateTime;

@Data
public class TenderPageQueryDTO implements Serializable {
    private int page = 1;
    private int size = 10;
    private String bidName;
    private String fileCategory; // bid/contract
    private Integer status;
    private LocalDate uploadStartTime;
    private LocalDate uploadEndTime;
}
