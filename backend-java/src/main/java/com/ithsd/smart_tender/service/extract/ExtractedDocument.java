package com.ithsd.smart_tender.service.extract;

import com.ithsd.smart_tender.service.extract.model.ParsedBlock;
import com.ithsd.smart_tender.service.chunking.ChunkSlice;

import java.util.ArrayList;
import java.util.List;

public class ExtractedDocument {
    private Long bidId;
    private String content;
    private String source;
    private boolean degraded;
    private Integer errorCode;
    private String errorMessage;
    private List<ParsedBlock> parsedBlocks = new ArrayList<>();
    private List<ChunkSlice> chunks = new ArrayList<>();

    public Long getBidId() {
        return bidId;
    }

    public void setBidId(Long bidId) {
        this.bidId = bidId;
    }

    public String getContent() {
        return content;
    }

    public void setContent(String content) {
        this.content = content;
    }

    public String getSource() {
        return source;
    }

    public void setSource(String source) {
        this.source = source;
    }

    public boolean isDegraded() {
        return degraded;
    }

    public void setDegraded(boolean degraded) {
        this.degraded = degraded;
    }

    public Integer getErrorCode() {
        return errorCode;
    }

    public void setErrorCode(Integer errorCode) {
        this.errorCode = errorCode;
    }

    public String getErrorMessage() {
        return errorMessage;
    }

    public void setErrorMessage(String errorMessage) {
        this.errorMessage = errorMessage;
    }

    public List<ParsedBlock> getParsedBlocks() {
        return parsedBlocks;
    }

    public void setParsedBlocks(List<ParsedBlock> parsedBlocks) {
        this.parsedBlocks = parsedBlocks;
    }

    public List<ChunkSlice> getChunks() {
        return chunks;
    }

    public void setChunks(List<ChunkSlice> chunks) {
        this.chunks = chunks;
    }
}
