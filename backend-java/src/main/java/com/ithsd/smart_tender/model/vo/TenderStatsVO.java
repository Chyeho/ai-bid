package com.ithsd.smart_tender.model.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class TenderStatsVO implements Serializable {
    private Long allCount;        // 全部标书数量
    private Long unreviewedCount; // 未审核
    private Long completedCount;  // 已审核
}
