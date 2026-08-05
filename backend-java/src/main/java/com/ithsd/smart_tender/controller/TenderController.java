package com.ithsd.smart_tender.controller;

import com.ithsd.smart_tender.model.dto.TenderDTO;
import com.ithsd.smart_tender.model.dto.TenderPageQueryDTO;
import com.ithsd.smart_tender.model.result.PageResult;
import com.ithsd.smart_tender.model.result.Result;
import com.ithsd.smart_tender.model.vo.TenderProjectVO;
import com.ithsd.smart_tender.model.vo.TenderStatsVO;
import com.ithsd.smart_tender.model.vo.TenderVO;
import com.ithsd.smart_tender.service.TenderService;
import com.ithsd.smart_tender.service.DocumentPreviewService;
import com.ithsd.smart_tender.service.StoragePathService;
import lombok.RequiredArgsConstructor;
import org.springframework.web.bind.annotation.*;
import org.springframework.web.multipart.MultipartFile;

import java.util.List;

@RestController
@RequestMapping("/api/bid-documents")
@RequiredArgsConstructor
public class TenderController {

    private final TenderService tenderService;
    private final DocumentPreviewService documentPreviewService;
    private final StoragePathService storagePathService;

    /**
     * 上传标书（新增版本参数）
     * @param file
     * @param tenderDTO
     * @return
     */
    @PostMapping("/upload")
    public Result<TenderVO> upload(@RequestParam("file") MultipartFile file, TenderDTO tenderDTO) {
        if (file.isEmpty()) {
            return Result.error(400, "上传文件不能为空");
        }
        TenderVO vo = tenderService.upload(file, tenderDTO);
        return Result.success(vo);
    }

    /**
     * 分页查询标书列表
     * @param tenderPageQueryDTO
     * @return
     */
    @GetMapping
    public Result<PageResult> page(TenderPageQueryDTO tenderPageQueryDTO) {
        PageResult pageResult = tenderService.page(tenderPageQueryDTO);
        return Result.success(pageResult);
    }

    /**
     * 条件获取标书统计数据
     * @param tenderPageQueryDTO
     * @return
     */
    @GetMapping("/stats")
    public Result<TenderStatsVO> getStats(TenderPageQueryDTO tenderPageQueryDTO) {
        TenderStatsVO vo = tenderService.getStats(tenderPageQueryDTO);
        return Result.success(vo);
    }

    /**
     * 查询某个标书详情
     * @param id
     * @return
     */
    @GetMapping("/{id}")
    public Result<TenderVO> getById(@PathVariable Long id) {
        TenderVO vo = tenderService.getById(id);
        return Result.success(vo);
    }

    /**
     * 获取某个项目下的所有版本
     * @param projectId
     * @return
     */
    @GetMapping("/project/{projectId}/versions")
    public Result<List<TenderVO>> getVersionsByProjectId(@PathVariable Long projectId) {
        java.util.List<TenderVO> list = tenderService.getVersionsByProjectId(projectId);
        return Result.success(list);
    }

    /**
     * 获取所有标书项目列表（聚合视图）
     * @return
     */
    @GetMapping("/projects")
    public Result<List<TenderProjectVO>> getProjects() {
        List<TenderProjectVO> list = tenderService.getProjects();
        return Result.success(list);
    }
    
    /**
     * 删除某个标书
     * @param id
     * @return
     */
    @DeleteMapping("/{id}")
    public Result delete(@PathVariable Long id) {
        tenderService.delete(id);
        return Result.success();
    }

    /**
     * 下载或预览标书文件
     * @param id
     * @return
     */
    @GetMapping("/{id}/download")
    public org.springframework.http.ResponseEntity<?> downloadFile(@PathVariable Long id) {
        TenderVO tender = tenderService.getById(id);
        if (tender == null || tender.getFilePath() == null) {
            return org.springframework.http.ResponseEntity.notFound().build();
        }

        String dbPath = tender.getFilePath();
        java.nio.file.Path path = storagePathService.resolveStoredPath(dbPath);

        String fileType = tender.getFileType() == null ? "" : tender.getFileType().toLowerCase();

        try {
            if ("pdf".equals(fileType)) {
                org.springframework.core.io.Resource resource = new org.springframework.core.io.UrlResource(path.toUri());
                if (!resource.exists() || !resource.isReadable()) {
                    return org.springframework.http.ResponseEntity.notFound().build();
                }
                return org.springframework.http.ResponseEntity.ok()
                        .header(org.springframework.http.HttpHeaders.CONTENT_DISPOSITION,
                                "inline; filename*=UTF-8''" + java.net.URLEncoder.encode(
                                        tender.getFileName(),
                                        java.nio.charset.StandardCharsets.UTF_8
                                ).replace("+", "%20"))
                        .contentType(org.springframework.http.MediaType.APPLICATION_PDF)
                        .body(resource);
            }

            if ("word".equals(fileType) || tender.getFileName().toLowerCase().endsWith(".docx")) {
                String downloadName = tender.getBidName() == null ? tender.getFileName() : tender.getBidName();
                java.nio.file.Path pdfPath = documentPreviewService.ensurePdfPreviewFile(path);
                org.springframework.core.io.Resource resource = new org.springframework.core.io.UrlResource(pdfPath.toUri());
                if (!resource.exists() || !resource.isReadable()) {
                    return org.springframework.http.ResponseEntity.notFound().build();
                }
                return org.springframework.http.ResponseEntity.ok()
                        .header(org.springframework.http.HttpHeaders.CONTENT_DISPOSITION,
                                "inline; filename*=UTF-8''" + java.net.URLEncoder.encode(
                                        downloadName + ".pdf",
                                        java.nio.charset.StandardCharsets.UTF_8
                                ).replace("+", "%20"))
                        .contentType(org.springframework.http.MediaType.APPLICATION_PDF)
                        .body(resource);
            }
        } catch (Exception ignored) {
            // 转换失败时降级为原始文件下载
        }

        try {
            org.springframework.core.io.Resource resource = new org.springframework.core.io.UrlResource(path.toUri());
            if (!resource.exists() || !resource.isReadable()) {
                return org.springframework.http.ResponseEntity.notFound().build();
            }

            return org.springframework.http.ResponseEntity.ok()
                    .header(org.springframework.http.HttpHeaders.CONTENT_DISPOSITION,
                            "attachment; filename*=UTF-8''" + java.net.URLEncoder.encode(
                                    tender.getFileName(),
                                    java.nio.charset.StandardCharsets.UTF_8
                            ).replace("+", "%20"))
                    .contentType(org.springframework.http.MediaType.APPLICATION_OCTET_STREAM)
                    .body(resource);
        } catch (Exception e) {
            return org.springframework.http.ResponseEntity.internalServerError().build();
        }
    }

    /*@PutMapping
    public Result update(@RequestBody TenderDTO tenderDTO) {
        tenderService.update(tenderDTO);
        return Result.success();
    }*/
}
