package com.ithsd.smart_tender.service.llm;

import org.springframework.core.io.ClassPathResource;
import org.springframework.stereotype.Component;
import org.springframework.util.StreamUtils;
import org.springframework.util.StringUtils;

import java.io.IOException;
import java.nio.charset.StandardCharsets;

@Component
public class PromptLoader {
    public String load(String checkType, String version) {
        if (!StringUtils.hasText(checkType) || !StringUtils.hasText(version)) {
            throw new IllegalArgumentException("invalid prompt key");
        }
        String path = "prompts/" + checkType + "_" + version + ".txt";
        ClassPathResource resource = new ClassPathResource(path);
        if (!resource.exists()) {
            throw new IllegalArgumentException("prompt template not found: " + path);
        }
        try {
            return StreamUtils.copyToString(resource.getInputStream(), StandardCharsets.UTF_8);
        } catch (IOException e) {
            throw new IllegalStateException("prompt template read failed: " + path, e);
        }
    }
}
