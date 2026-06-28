package com.ithsd.smart_tender.service.chunking;

import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

@Component
public class StableIdGenerator {

    public String generate(String text, String titlePath, String anchor) {
        String normalizedText = normalize(text);
        String normalizedTitlePath = normalize(titlePath);
        String normalizedAnchor = normalize(anchor);
        String source = normalizedText + "|" + normalizedTitlePath + "|" + normalizedAnchor;
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] hash = digest.digest(source.getBytes(StandardCharsets.UTF_8));
            StringBuilder builder = new StringBuilder();
            for (byte value : hash) {
                builder.append(String.format("%02x", value));
            }
            return builder.toString();
        } catch (NoSuchAlgorithmException ex) {
            throw new IllegalStateException("SHA-256_NOT_SUPPORTED");
        }
    }

    public String normalize(String text) {
        if (!StringUtils.hasText(text)) {
            return "";
        }
        return text.toLowerCase()
                .replace('\u00A0', ' ')
                .replaceAll("\\s+", " ")
                .replaceAll("[\\p{Punct}，。！？；：“”‘’（）【】《》、]+", "")
                .trim();
    }
}
