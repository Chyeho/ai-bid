package com.ithsd.smart_tender.service.llm;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.ithsd.smart_tender.pojo.entity.AuditIssueEntity;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.time.LocalDateTime;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

@Component
public class JsonLinesIssueParser {
    private static final ObjectMapper OBJECT_MAPPER = new ObjectMapper();
    private static final Set<String> ALLOWED_CATEGORIES = new HashSet<>(List.of("budget", "demand", "legal"));

    public List<AuditIssueEntity> parse(String checkType, String jsonLines) {
        List<AuditIssueEntity> issues = new ArrayList<>();
        if (!StringUtils.hasText(jsonLines)) {
            return issues;
        }
        String[] lines = jsonLines.split("\\r?\\n");
        for (String line : lines) {
            if (!StringUtils.hasText(line)) {
                continue;
            }
            try {
                JsonNode root = OBJECT_MAPPER.readTree(line);
                AuditIssueEntity issue = new AuditIssueEntity();
                issue.setSeverity(normalizeSeverity(root.path("severity").asText()));
                issue.setCategory(normalizeCategory(checkType, root.path("category").asText()));
                issue.setDescription(defaultIfBlank(root.path("description").asText(), "待补充问题描述"));
                issue.setSuggestion(defaultIfBlank(root.path("suggestion").asText(), "请人工复核"));
                issue.setReference(defaultIfBlank(root.path("reference").asText(), "stub_llm"));
                JsonNode location = root.path("location");
                if (location.isObject()) {
                    issue.setPageNumber(location.path("pageNumber").isInt() ? location.path("pageNumber").asInt() : null);
                    issue.setSectionName(defaultIfBlank(location.path("sectionName").asText(), null));
                    issue.setContext(defaultIfBlank(location.path("context").asText(), null));
                }
                issue.setCreateTime(LocalDateTime.now());
                issues.add(issue);
            } catch (Exception ignored) {
            }
        }
        return issues;
    }

    private String normalizeSeverity(String severity) {
        if ("critical".equals(severity) || "warning".equals(severity) || "info".equals(severity)) {
            return severity;
        }
        return "warning";
    }

    private String normalizeCategory(String checkType, String category) {
        if (StringUtils.hasText(category) && ALLOWED_CATEGORIES.contains(category)) {
            return category;
        }
        if (StringUtils.hasText(checkType) && ALLOWED_CATEGORIES.contains(checkType)) {
            return checkType;
        }
        return "budget";
    }

    private String defaultIfBlank(String value, String defaultValue) {
        if (StringUtils.hasText(value)) {
            return value;
        }
        return defaultValue;
    }
}
