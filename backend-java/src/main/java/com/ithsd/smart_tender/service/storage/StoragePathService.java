package com.ithsd.smart_tender.service.storage;

import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import org.springframework.util.StringUtils;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

@Service
public class StoragePathService {

    @Value("${file.storage.path}")
    private String storageRoot;

    @Value("${file.storage.tender-dir:tenders}")
    private String tenderDir;

    @Value("${file.storage.knowledge-dir:knowledge/uploads}")
    private String knowledgeDir;

    @Value("${preview.cache.path:}")
    private String previewCacheDir;

    public Path rootPath() {
        return Paths.get(storageRoot).normalize();
    }

    public Path tenderRootPath() {
        return rootPath().resolve(tenderDir).normalize();
    }

    public Path knowledgeRootPath() {
        return rootPath().resolve(knowledgeDir).normalize();
    }

    public Path previewCachePath() {
        if (StringUtils.hasText(previewCacheDir)) {
            return Paths.get(previewCacheDir).normalize();
        }
        return rootPath().resolve("preview-cache").normalize();
    }

    public Path buildTenderUploadPath(String originalFilename) {
        return buildUploadPath(tenderRootPath(), originalFilename);
    }

    public Path buildKnowledgeUploadPath(String originalFilename) {
        return buildUploadPath(knowledgeRootPath(), originalFilename);
    }

    public String toStoredPath(Path absolutePath) {
        Path normalized = absolutePath.toAbsolutePath().normalize();
        Path root = rootPath().toAbsolutePath().normalize();
        if (normalized.startsWith(root)) {
            String relative = root.relativize(normalized).toString();
            return relative.replace("\\", "/");
        }
        return normalized.toString().replace("\\", "/");
    }

    public Path resolveStoredPath(String storedPath) {
        if (!StringUtils.hasText(storedPath)) {
            return null;
        }
        String normalizedRaw = storedPath.trim().replace("\\", "/");

        Path direct = Paths.get(storedPath).toAbsolutePath().normalize();
        if (direct.isAbsolute() && Files.exists(direct)) {
            return direct;
        }

        Path root = rootPath().toAbsolutePath().normalize();
        List<Path> candidates = new ArrayList<>();
        candidates.add(root.resolve(normalizedRaw).normalize());
        candidates.add(knowledgeRootPath().resolve(normalizedRaw).normalize());
        candidates.add(tenderRootPath().resolve(normalizedRaw).normalize());

        if (normalizedRaw.startsWith("data/uploads/")) {
            candidates.add(knowledgeRootPath().resolve(normalizedRaw.substring("data/uploads/".length())).normalize());
        }
        if (normalizedRaw.startsWith("uploads/")) {
            candidates.add(knowledgeRootPath().resolve(normalizedRaw.substring("uploads/".length())).normalize());
        }
        if (normalizedRaw.startsWith("tenders/")) {
            candidates.add(tenderRootPath().resolve(normalizedRaw.substring("tenders/".length())).normalize());
        }

        for (Path candidate : candidates) {
            if (Files.exists(candidate)) {
                return candidate;
            }
        }
        return candidates.get(0);
    }

    public void ensureParentDirectory(Path path) throws IOException {
        Path parent = path.getParent();
        if (parent != null && !Files.exists(parent)) {
            Files.createDirectories(parent);
        }
    }

    private Path buildUploadPath(Path baseDir, String originalFilename) {
        String extension = "";
        if (StringUtils.hasText(originalFilename)) {
            int dot = originalFilename.lastIndexOf(".");
            if (dot >= 0) {
                extension = originalFilename.substring(dot);
            }
        }
        String dateFolder = LocalDate.now().format(DateTimeFormatter.ofPattern("yyyy-MM-dd"));
        String uniqueFileName = UUID.randomUUID() + extension;
        return baseDir.resolve(dateFolder).resolve(uniqueFileName).toAbsolutePath().normalize();
    }
}
