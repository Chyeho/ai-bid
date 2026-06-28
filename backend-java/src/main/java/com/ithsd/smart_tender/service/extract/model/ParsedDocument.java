package com.ithsd.smart_tender.service.extract.model;

import java.util.ArrayList;
import java.util.List;

public class ParsedDocument {
    private Long bidId;
    private String fileType;
    private List<ParsedBlock> blocks = new ArrayList<>();

    public Long getBidId() {
        return bidId;
    }

    public void setBidId(Long bidId) {
        this.bidId = bidId;
    }

    public String getFileType() {
        return fileType;
    }

    public void setFileType(String fileType) {
        this.fileType = fileType;
    }

    public List<ParsedBlock> getBlocks() {
        return blocks;
    }

    public void setBlocks(List<ParsedBlock> blocks) {
        this.blocks = blocks;
    }
}
