package com.ithsd.smart_tender.pojo.vo;

import java.util.List;

public class DocumentParseChunkPageVO {
    private Long total;
    private List<DocumentParseChunkVO> records;

    public Long getTotal() {
        return total;
    }

    public void setTotal(Long total) {
        this.total = total;
    }

    public List<DocumentParseChunkVO> getRecords() {
        return records;
    }

    public void setRecords(List<DocumentParseChunkVO> records) {
        this.records = records;
    }
}
