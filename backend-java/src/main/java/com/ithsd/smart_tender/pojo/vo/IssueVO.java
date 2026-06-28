package com.ithsd.smart_tender.pojo.vo;

import java.util.List;

public class IssueVO {
    private String issueNo;
    private String severity;
    private String category;
    private String dimension;
    private String description;
    private LocationVO location;
    private String suggestion;
    private String reference;
    private String anchorQuote;
    private Integer anchorPage;
    private String anchorSection;
    private List<String> anchorTokens;
    private List<Integer> anchorCharsRange;

    public String getIssueNo() {
        return issueNo;
    }

    public void setIssueNo(String issueNo) {
        this.issueNo = issueNo;
    }

    public String getSeverity() {
        return severity;
    }

    public void setSeverity(String severity) {
        this.severity = severity;
    }

    public String getCategory() {
        return category;
    }

    public void setCategory(String category) {
        this.category = category;
    }

    public String getDescription() {
        return description;
    }

    public void setDescription(String description) {
        this.description = description;
    }

    public String getDimension() {
        return dimension;
    }

    public void setDimension(String dimension) {
        this.dimension = dimension;
    }

    public LocationVO getLocation() {
        return location;
    }

    public void setLocation(LocationVO location) {
        this.location = location;
    }

    public String getSuggestion() {
        return suggestion;
    }

    public void setSuggestion(String suggestion) {
        this.suggestion = suggestion;
    }

    public String getReference() {
        return reference;
    }

    public void setReference(String reference) {
        this.reference = reference;
    }

    public String getAnchorQuote() {
        return anchorQuote;
    }

    public void setAnchorQuote(String anchorQuote) {
        this.anchorQuote = anchorQuote;
    }

    public Integer getAnchorPage() {
        return anchorPage;
    }

    public void setAnchorPage(Integer anchorPage) {
        this.anchorPage = anchorPage;
    }

    public String getAnchorSection() {
        return anchorSection;
    }

    public void setAnchorSection(String anchorSection) {
        this.anchorSection = anchorSection;
    }

    public List<String> getAnchorTokens() {
        return anchorTokens;
    }

    public void setAnchorTokens(List<String> anchorTokens) {
        this.anchorTokens = anchorTokens;
    }

    public List<Integer> getAnchorCharsRange() {
        return anchorCharsRange;
    }

    public void setAnchorCharsRange(List<Integer> anchorCharsRange) {
        this.anchorCharsRange = anchorCharsRange;
    }

    public static class LocationVO {
        private Integer pageNumber;
        private String sectionName;
        private String context;

        public Integer getPageNumber() {
            return pageNumber;
        }

        public void setPageNumber(Integer pageNumber) {
            this.pageNumber = pageNumber;
        }

        public String getSectionName() {
            return sectionName;
        }

        public void setSectionName(String sectionName) {
            this.sectionName = sectionName;
        }

        public String getContext() {
            return context;
        }

        public void setContext(String context) {
            this.context = context;
        }
    }
}
