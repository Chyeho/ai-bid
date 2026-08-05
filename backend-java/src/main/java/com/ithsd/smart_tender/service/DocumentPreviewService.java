package com.ithsd.smart_tender.service;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.core.io.ByteArrayResource;
import org.springframework.http.HttpEntity;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.stereotype.Service;
import org.springframework.util.LinkedMultiValueMap;
import org.springframework.util.MultiValueMap;
import org.springframework.web.client.RestTemplate;

import java.io.IOException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;

@Service
public class DocumentPreviewService {

    @Value("${preview.converter.base-url}")
    private String converterBaseUrl;

    @Value("${preview.cache.path:}")
    private String previewCachePath;

    public ResponseEntity<ByteArrayResource> convertDocxToPdf(Path sourcePath, String downloadFileName) throws IOException {
        Path pdfPath = ensurePdfPreviewFile(sourcePath);
        byte[] pdfBytes = Files.readAllBytes(pdfPath);
        ByteArrayResource pdfResource = new ByteArrayResource(pdfBytes);
        String finalName = (downloadFileName == null || downloadFileName.isBlank())
                ? "document.pdf"
                : downloadFileName + ".pdf";
        String encodedName = URLEncoder.encode(finalName, StandardCharsets.UTF_8).replace("+", "%20");

        return ResponseEntity.ok()
                .header(HttpHeaders.CONTENT_DISPOSITION, "inline; filename*=UTF-8''" + encodedName)
                .contentType(MediaType.APPLICATION_PDF)
                .body(pdfResource);
    }

    public Path ensurePdfPreviewFile(Path sourcePath) throws IOException {
        String name = sourcePath.getFileName().toString().toLowerCase();
        if (name.endsWith(".pdf")) {
            return sourcePath;
        }
        if (!name.endsWith(".doc") && !name.endsWith(".docx")) {
            throw new IOException("仅支持Word文档转PDF: " + sourcePath);
        }
        Path target = buildPreviewPdfPath(sourcePath);
        if (Files.exists(target) && Files.getLastModifiedTime(target).toMillis() >= Files.getLastModifiedTime(sourcePath).toMillis()) {
            return target;
        }
        byte[] pdfBytes = convertToPdfBytes(sourcePath);
        Files.write(target, pdfBytes, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
        return target;
    }

    public byte[] convertToPdfBytes(Path sourcePath) throws IOException {
        byte[] fileBytes = Files.readAllBytes(sourcePath);

        RestTemplate restTemplate = new RestTemplate();

        HttpHeaders headers = new HttpHeaders();
        // Do NOT explicitly set Content-Type to MediaType.MULTIPART_FORM_DATA!
        // Spring's RestTemplate uses FormHttpMessageConverter which will automatically
        // set the correct boundary if we don't override the content type here.

        MultiValueMap<String, Object> body = new LinkedMultiValueMap<>();
        
        ByteArrayResource resource = new ByteArrayResource(fileBytes) {
            @Override
            public String getFilename() {
                return sourcePath.getFileName().toString();
            }
        };
        
        body.add("data", resource); // JODConverter REST expects multipart field name "data"

        HttpEntity<MultiValueMap<String, Object>> requestEntity = new HttpEntity<>(body, headers);

        String url = converterBaseUrl + "/lool/convert-to/pdf";

        ResponseEntity<byte[]> response;
        try {
            response = restTemplate.postForEntity(url, requestEntity, byte[].class);
        } catch (org.springframework.web.client.RestClientException e) {
            throw new IOException("文档转换失败，API 调用异常: " + e.getMessage(), e);
        }

        if (!response.getStatusCode().is2xxSuccessful() || response.getBody() == null) {
            throw new IOException("文档转换失败，状态码: " + response.getStatusCode());
        }
        return response.getBody();
    }

    private Path buildPreviewPdfPath(Path sourcePath) throws IOException {
        Path cacheRoot;
        if (previewCachePath == null || previewCachePath.isBlank()) {
            cacheRoot = sourcePath.getParent().resolve(".preview-cache");
        } else {
            cacheRoot = Paths.get(previewCachePath);
        }
        Files.createDirectories(cacheRoot);
        String cacheKey = buildCacheKey(sourcePath);
        return cacheRoot.resolve(cacheKey + ".preview.pdf");
    }

    private String buildCacheKey(Path sourcePath) throws IOException {
        String seed = sourcePath.toAbsolutePath().normalize() + "|" + Files.getLastModifiedTime(sourcePath).toMillis();
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] bytes = digest.digest(seed.getBytes(StandardCharsets.UTF_8));
            StringBuilder builder = new StringBuilder();
            for (byte value : bytes) {
                builder.append(String.format("%02x", value));
            }
            return builder.substring(0, 24);
        } catch (NoSuchAlgorithmException ex) {
            throw new IOException("预览缓存键生成失败", ex);
        }
    }
}
