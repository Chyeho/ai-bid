package com.ithsd.smart_tender.pojo.vo;

import java.time.LocalDateTime;
import java.util.Map;

public class DocumentParseChunkVO {
    private String chunkId;
    private String stableId;
    private String stableIdVersion;
    private String strategyVersion;
    private Integer chunkIndex;
    private String content;
    private Integer length;
    private String titlePath;
    private Integer pageStart;
    private Integer pageEnd;
    private Map<String, Object> anchor;
    private LocalDateTime createdAt;

    public String getChunkId() {
        return chunkId;
    }

    public void setChunkId(String chunkId) {
        this.chunkId = chunkId;
    }

    public String getStableId() {
        return stableId;
    }

    public void setStableId(String stableId) {
        this.stableId = stableId;
    }

    public String getStableIdVersion() {
        return stableIdVersion;
    }

    public void setStableIdVersion(String stableIdVersion) {
        this.stableIdVersion = stableIdVersion;
    }

    public String getStrategyVersion() {
        return strategyVersion;
    }

    public void setStrategyVersion(String strategyVersion) {
        this.strategyVersion = strategyVersion;
    }

    public Integer getChunkIndex() {
        return chunkIndex;
    }

    public void setChunkIndex(Integer chunkIndex) {
        this.chunkIndex = chunkIndex;
    }

    public String getContent() {
        return content;
    }

    public void setContent(String content) {
        this.content = content;
    }

    public Integer getLength() {
        return length;
    }

    public void setLength(Integer length) {
        this.length = length;
    }

    public String getTitlePath() {
        return titlePath;
    }

    public void setTitlePath(String titlePath) {
        this.titlePath = titlePath;
    }

    public Integer getPageStart() {
        return pageStart;
    }

    public void setPageStart(Integer pageStart) {
        this.pageStart = pageStart;
    }

    public Integer getPageEnd() {
        return pageEnd;
    }

    public void setPageEnd(Integer pageEnd) {
        this.pageEnd = pageEnd;
    }

    public Map<String, Object> getAnchor() {
        return anchor;
    }

    public void setAnchor(Map<String, Object> anchor) {
        this.anchor = anchor;
    }

    public LocalDateTime getCreatedAt() {
        return createdAt;
    }

    public void setCreatedAt(LocalDateTime createdAt) {
        this.createdAt = createdAt;
    }
}
