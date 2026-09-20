package com.ithsd.smart_tender.service;

import org.springframework.core.io.ByteArrayResource;
import org.springframework.http.HttpHeaders;
import org.springframework.http.MediaType;
import org.springframework.http.ResponseEntity;
import org.springframework.stereotype.Service;

import java.io.IOException;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.nio.file.AtomicMoveNotSupportedException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.nio.file.StandardCopyOption;
import java.nio.file.StandardOpenOption;
import java.nio.file.attribute.PosixFilePermissions;
import java.util.Comparator;
import java.util.List;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;

@Service
public class DocumentPreviewService {

    /** 最大并发转换数：LibreOffice headless 单实例较重，限制并发避免内存/CPU 尖峰。 */
    private static final int MAX_CONCURRENT_CONVERSIONS = 2;

    /** 等待转换名额的最长时间（秒）。 */
    private static final long CONVERT_WAIT_SECONDS = 30;

    /** 单次 soffice 转换超时（秒），超时后强制终止进程。 */
    private static final long CONVERT_TIMEOUT_SECONDS = 60;

    /** soffice 以该非 root 系统用户运行（安全加固；见 Dockerfile 中创建的同名用户）。 */
    private static final String SOFFICE_USER = "soffice";

    private static final Semaphore CONVERSION_LIMIT = new Semaphore(MAX_CONCURRENT_CONVERSIONS);

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

    /**
     * 确保 Word 文档存在<b>持久化的同目录 PDF 版本</b>（sibling：{@code <stem>.pdf}），返回其路径。
     *
     * <p><b>链路唯一转换点：</b>全系统只保留这一处 LibreOffice（Java 容器）。本方法产出的
     * PDF 同时用于：① 前端预览/下载；② 上传 Rust 审查引擎（提取高亮坐标）；③ 知识库 ingest。
     * 审查高亮坐标与预览 PDF 必须出自同一份文件——历史上 Java/Rust 各转一次 PDF，
     * 两套 LibreOffice 版本不同导致分页/断行不一致，高亮整体错位。</p>
     *
     * <p>幂等：目标 PDF 已存在且不早于源文件 mtime 时直接复用；否则现场转换。
     * 写入采用「同目录临时文件 + 原子改名」，避免并发读者读到半成品。</p>
     */
    public Path ensurePdfPreviewFile(Path sourcePath) throws IOException {
        String name = sourcePath.getFileName().toString().toLowerCase();
        if (name.endsWith(".pdf")) {
            return sourcePath;
        }
        if (!name.endsWith(".doc") && !name.endsWith(".docx")) {
            throw new IOException("仅支持Word文档转PDF: " + sourcePath);
        }
        Path target = siblingPdfPath(sourcePath);
        if (Files.exists(target) && Files.getLastModifiedTime(target).toMillis() >= Files.getLastModifiedTime(sourcePath).toMillis()) {
            return target;
        }
        byte[] pdfBytes = convertToPdfBytes(sourcePath);
        Path parent = target.getParent();
        if (parent != null) {
            Files.createDirectories(parent);
        }
        Path tmp = Files.createTempFile(parent, target.getFileName().toString() + ".", ".tmp");
        try {
            Files.write(tmp, pdfBytes, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
            try {
                Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
            } catch (AtomicMoveNotSupportedException e) {
                Files.move(tmp, target, StandardCopyOption.REPLACE_EXISTING);
            }
        } finally {
            Files.deleteIfExists(tmp);
        }
        return target;
    }

    /** 源文件同目录的 sibling PDF 路径：{@code <stem>.pdf}（同目录 → 天然属于同一租户空间）。 */
    private Path siblingPdfPath(Path sourcePath) {
        String name = sourcePath.getFileName().toString();
        int dot = name.lastIndexOf('.');
        String stem = dot > 0 ? name.substring(0, dot) : name;
        return sourcePath.resolveSibling(stem + ".pdf");
    }

    /**
     * 使用本地 LibreOffice（soffice headless）将 .doc/.docx 转为 PDF 字节。
     *
     * <p>相比原先调用 JODConverter REST 服务（需额外部署 doc-converter 容器），
     * 这里直接调用容器内已安装的 LibreOffice，避免额外服务依赖。</p>
     *
     * <p>关键防护：每次转换使用独立的 UserInstallation profile，规避并发时的
     * profile 锁冲突；用信号量限制并发数；对进程设置超时并强制终止；
     * soffice 以非 root 系统用户（{@code soffice}）运行，降低处理不可信文档的权限。</p>
     */
    public byte[] convertToPdfBytes(Path sourcePath) throws IOException {
        String name = sourcePath.getFileName().toString().toLowerCase();
        if (!name.endsWith(".doc") && !name.endsWith(".docx")) {
            throw new IOException("仅支持Word文档转PDF: " + sourcePath);
        }

        Path tempDir = Files.createTempDirectory("lo-convert-");
        try {
            boolean acquired = false;
            try {
                acquired = CONVERSION_LIMIT.tryAcquire(CONVERT_WAIT_SECONDS, TimeUnit.SECONDS);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
                throw new IOException("等待文档转换资源时被中断", e);
            }
            if (!acquired) {
                throw new IOException("文档转换服务繁忙，请稍后重试");
            }
            try {
                return runSofficeConversion(sourcePath, tempDir);
            } finally {
                CONVERSION_LIMIT.release();
            }
        } finally {
            deleteRecursively(tempDir);
        }
    }

    private byte[] runSofficeConversion(Path sourcePath, Path tempDir) throws IOException {
        Path outDir = Files.createDirectories(tempDir.resolve("out"));
        Path profileDir = Files.createDirectories(tempDir.resolve("profile"));
        Path logFile = tempDir.resolve("soffice.log");

        // soffice 以非 root 用户运行，需让其可写这些临时工作目录（随机目录名，用完即删）
        makeWorldAccessible(tempDir, outDir, profileDir);

        List<String> command = List.of(
                "runuser", "-u", SOFFICE_USER, "--",
                resolveSoffice(),
                "--headless",
                "--nologo",
                "--nofirststartwizard",
                "--nodefault",
                "-env:UserInstallation=file://" + profileDir.toAbsolutePath(),
                "--convert-to", "pdf",
                "--outdir", outDir.toAbsolutePath().toString(),
                sourcePath.toAbsolutePath().toString()
        );

        ProcessBuilder builder = new ProcessBuilder(command);
        builder.redirectErrorStream(true);
        builder.redirectOutput(logFile.toFile());

        Process process;
        try {
            process = builder.start();
        } catch (IOException e) {
            throw new IOException("无法启动 LibreOffice (soffice)，请确认已安装: " + e.getMessage(), e);
        }

        boolean finished;
        try {
            finished = process.waitFor(CONVERT_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            killProcessTree(process);
            throw new IOException("等待 LibreOffice 转换时被中断", e);
        }

        if (!finished) {
            killProcessTree(process);
            throw new IOException("文档转换超时（超过 " + CONVERT_TIMEOUT_SECONDS + " 秒），已终止转换进程");
        }

        if (process.exitValue() != 0) {
            String log = readLog(logFile);
            throw new IOException("文档转换失败，退出码 " + process.exitValue() + (log.isBlank() ? "" : ": " + log));
        }

        Path pdf = outDir.resolve(stemOf(sourcePath) + ".pdf");
        if (!Files.exists(pdf)) {
            throw new IOException("LibreOffice 未生成 PDF 文件: " + pdf);
        }
        return Files.readAllBytes(pdf);
    }

    private String resolveSoffice() throws IOException {
        String env = System.getenv("LIBREOFFICE_PATH");
        if (env != null && !env.isBlank()) {
            Path candidate = Paths.get(env);
            if (Files.isExecutable(candidate)) {
                return candidate.toAbsolutePath().toString();
            }
        }
        // 依赖 PATH（Linux 容器内通常为 /usr/bin/soffice）
        return "soffice";
    }

    private void makeWorldAccessible(Path... paths) throws IOException {
        for (Path path : paths) {
            Files.setPosixFilePermissions(path, PosixFilePermissions.fromString("rwxrwxrwx"));
        }
    }

    private void killProcessTree(Process process) {
        try {
            process.descendants().forEach(handle -> {
                try {
                    handle.destroyForcibly();
                } catch (Exception ignored) {
                }
            });
        } catch (Exception ignored) {
        }
        process.destroyForcibly();
        try {
            process.waitFor(2, TimeUnit.SECONDS);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
    }

    private String readLog(Path logFile) {
        try {
            if (Files.exists(logFile)) {
                String content = Files.readString(logFile, StandardCharsets.UTF_8).trim();
                if (content.length() > 2000) {
                    content = content.substring(content.length() - 2000);
                }
                return content;
            }
        } catch (IOException ignored) {
        }
        return "";
    }

    private String stemOf(Path path) {
        String name = path.getFileName().toString();
        int dot = name.lastIndexOf('.');
        return dot > 0 ? name.substring(0, dot) : name;
    }

    private void deleteRecursively(Path root) {
        if (root == null || !Files.exists(root)) {
            return;
        }
        try (var stream = Files.walk(root)) {
            stream.sorted(Comparator.reverseOrder()).forEach(path -> {
                try {
                    Files.deleteIfExists(path);
                } catch (IOException ignored) {
                }
            });
        } catch (IOException ignored) {
        }
    }
}
