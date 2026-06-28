package com.ithsd.smart_tender.pojo.vo;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.io.Serializable;
import java.util.List;

@Data
@Builder
@NoArgsConstructor
@AllArgsConstructor
public class ChatResponseVO implements Serializable {
    private String content; // AI回复内容
    private List<Object> citations; // 引用来源
}
