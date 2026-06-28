package com.ithsd.smart_tender.service.impl;

import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.baomidou.mybatisplus.extension.plugins.pagination.Page;
import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.ithsd.smart_tender.exception.BizException;
import com.ithsd.smart_tender.mapper.KnowledgeChunkMapper;
import com.ithsd.smart_tender.mapper.KnowledgeFileMapper;
import com.ithsd.smart_tender.pojo.dto.CreateDocumentParseJobRequest;
import com.ithsd.smart_tender.pojo.entity.DocumentParseJobEntity;
import com.ithsd.smart_tender.pojo.entity.KnowledgeChunk;
import com.ithsd.smart_tender.pojo.entity.KnowledgeFile;
import com.ithsd.smart_tender.pojo.entity.RagTriggerOutboxEntity;
import com.ithsd.smart_tender.pojo.vo.CreateDocumentParseJobVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseChunkPageVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseChunkVO;
import com.ithsd.smart_tender.pojo.vo.DocumentParseJobStatusVO;
import com.ithsd.smart_tender.repository.DocumentParseJobRepository;
import com.ithsd.smart_tender.repository.RagTriggerOutboxRepository;
import com.ithsd.smart_tender.service.DocumentParseJobService;
import com.ithsd.smart_tender.service.KnowledgeChunkService;
import com.ithsd.smart_tender.service.chunking.ChunkingProperties;
import com.ithsd.smart_tender.service.storage.StoragePathService;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;

import java.io.IOException;
import java.time.LocalDateTime;
import java.util.Collection;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.UUID;
import java.nio.file.Path;

@Service
public class DocumentParseJobServiceImpl implements DocumentParseJobService {
    private static final Set<String> SUPPORTED_SOURCE_TYPES = Set.of("doc", "docx");
    private static final Set<String> SUPPORTED_PRIORITIES = Set.of("high", "normal", "low");
    private static final Collection<String> ACTIVE_STATUSES = List.of("pending", "processing");
    private static final int ERROR_MSG_MAX_LENGTH = 1000;

    private final DocumentParseJobRepository documentParseJobRepository;
    private final KnowledgeFileMapper knowledgeFileMapper;
    private final KnowledgeChunkService knowledgeChunkService;
    private final KnowledgeChunkMapper knowledgeChunkMapper;
    private final RagTriggerOutboxRepository ragTriggerOutboxRepository;
    private final ChunkingProperties chunkingProperties;
    private final StoragePathService storagePathService;
    private final ObjectMapper objectMapper;

    public DocumentParseJobServiceImpl(
            DocumentParseJobRepository documentParseJobRepository,
            KnowledgeFileMapper knowledgeFileMapper,
            KnowledgeChunkService knowledgeChunkService,
            KnowledgeChunkMapper knowledgeChunkMapper,
            RagTriggerOutboxRepository ragTriggerOutboxRepository,
            ChunkingProperties chunkingProperties,
            StoragePathService storagePathService
    ) {
        this.documentParseJobRepository = documentParseJobRepository;
        this.knowledgeFileMapper = knowledgeFileMapper;
        this.knowledgeChunkService = knowledgeChunkService;
        this.knowledgeChunkMapper = knowledgeChunkMapper;
        this.ragTriggerOutboxRepository = ragTriggerOutboxRepository;
        this.chunkingProperties = chunkingProperties;
        this.storagePathService = storagePathService;
        this.objectMapper = new ObjectMapper();
    }

    @Override
    @Transactional
    public CreateDocumentParseJobVO createJob(CreateDocumentParseJobRequest request) {
        if (StringUtils.hasText(request.getRequestId())) {
            Optional<DocumentParseJobEntity> existingByRequest = documentParseJobRepository.findTopByRequestIdOrderByCreatedAtDesc(request.getRequestId());
            if (existingByRequest.isPresent()) {
                return toCreateVO(existingByRequest.get());
            }
        }
        String sourceType = normalizeLower(request.getSourceType());
        if (!SUPPORTED_SOURCE_TYPES.contains(sourceType)) {
            throw new BizException(5611, "不支持的sourceType");
        }
        String priority = normalizeLower(request.getPriority());
        if (!SUPPORTED_PRIORITIES.contains(priority)) {
            throw new BizException(5610, "priority不合法");
        }
        Long fileId = parseFileId(request.getFileId());
        KnowledgeFile knowledgeFile = knowledgeFileMapper.selectById(fileId);
        if (knowledgeFile == null || Integer.valueOf(2).equals(knowledgeFile.getStatus())) {
            throw new BizException(5612, "文件不存在");
        }
        String fileType = normalizeLower(knowledgeFile.getFileType());
        if (StringUtils.hasText(fileType) && !sourceType.equals(fileType)) {
            throw new BizException(5611, "sourceType与文件类型不匹配");
        }
        String strategyVersion = StringUtils.hasText(request.getStrategyVersion()) ? request.getStrategyVersion() : "chunk-v1";
        List<DocumentParseJobEntity> activeJobs = documentParseJobRepository.findByFileIdAndStrategyVersionAndStatusIn(fileId, strategyVersion, ACTIVE_STATUSES);
        if (!activeJobs.isEmpty()) {
            throw new BizException(5613, "存在进行中的任务");
        }
        LocalDateTime now = LocalDateTime.now();
        DocumentParseJobEntity entity = new DocumentParseJobEntity();
        entity.setJobId(buildJobId());
        entity.setRequestId(request.getRequestId());
        entity.setFileId(fileId);
        entity.setFileName(StringUtils.hasText(request.getFileName()) ? request.getFileName() : knowledgeFile.getFileName());
        entity.setSourceType(sourceType);
        entity.setPriority(priority);
        entity.setTriggerRag(request.getTriggerRag() == null ? Boolean.TRUE : request.getTriggerRag());
        entity.setStrategyVersion(strategyVersion);
        entity.setStatus("pending");
        entity.setStage("queued");
        entity.setProgress(0);
        entity.setChunkCount(0);
        entity.setCreatedAt(now);
        entity.setUpdatedAt(now);
        entity.setFailedStages(List.of());
        DocumentParseJobEntity saved = documentParseJobRepository.save(entity);
        processJob(saved, knowledgeFile.getFilePath());
        return toCreateVO(saved);
    }

    @Override
    @Transactional(readOnly = true)
    public DocumentParseJobStatusVO getStatus(String jobId) {
        DocumentParseJobEntity entity = loadJob(jobId);
        DocumentParseJobStatusVO vo = new DocumentParseJobStatusVO();
        vo.setJobId(entity.getJobId());
        vo.setFileId(String.valueOf(entity.getFileId()));
        vo.setStatus(entity.getStatus());
        vo.setStage(entity.getStage());
        vo.setProgress(entity.getProgress());
        vo.setChunkCount(entity.getChunkCount());
        vo.setFailedStages(entity.getFailedStages() == null ? List.of() : entity.getFailedStages());
        vo.setErrorMsg(entity.getErrorMsg());
        vo.setStrategyVersion(entity.getStrategyVersion());
        vo.setTriggerStatus(resolveTriggerStatus(entity));
        vo.setCreatedAt(entity.getCreatedAt());
        vo.setUpdatedAt(entity.getUpdatedAt());
        vo.setStartTime(entity.getStartTime());
        vo.setEndTime(entity.getEndTime());
        return vo;
    }

    @Override
    @Transactional(readOnly = true)
    public DocumentParseChunkPageVO listChunks(String jobId, Integer page, Integer size, String sinceChunkId) {
        DocumentParseJobEntity job = loadJob(jobId);
        Long sinceId = parseSinceChunkId(sinceChunkId);
        LambdaQueryWrapper<KnowledgeChunk> wrapper = new LambdaQueryWrapper<>();
        wrapper.eq(KnowledgeChunk::getFileId, job.getFileId());
        if (sinceId != null) {
            wrapper.gt(KnowledgeChunk::getId, sinceId);
        }
        wrapper.orderByAsc(KnowledgeChunk::getId);
        Page<KnowledgeChunk> mpPage = new Page<>(page, size);
        Page<KnowledgeChunk> result = knowledgeChunkMapper.selectPage(mpPage, wrapper);
        DocumentParseChunkPageVO pageVO = new DocumentParseChunkPageVO();
        pageVO.setTotal(result.getTotal());
        pageVO.setRecords(result.getRecords().stream().map(this::toChunkVO).toList());
        return pageVO;
    }

    private void processJob(DocumentParseJobEntity job, String filePath) {
        LocalDateTime start = LocalDateTime.now();
        job.setStatus("processing");
        job.setStage("chunking");
        job.setProgress(20);
        job.setStartTime(start);
        job.setUpdatedAt(start);
        documentParseJobRepository.save(job);
        try {
            Path absolutePath = storagePathService.resolveStoredPath(filePath);
            knowledgeChunkService.processFileChunks(job.getFileId(), absolutePath.toString(), "kb");
            Long count = knowledgeChunkMapper.selectCount(new LambdaQueryWrapper<KnowledgeChunk>().eq(KnowledgeChunk::getFileId, job.getFileId()));
            LocalDateTime end = LocalDateTime.now();
            job.setStatus("completed");
            job.setStage("completed");
            job.setProgress(100);
            job.setChunkCount(count == null ? 0 : Math.toIntExact(count));
            job.setErrorMsg(null);
            job.setFailedStages(List.of());
            job.setEndTime(end);
            job.setUpdatedAt(end);
        } catch (IOException e) {
            LocalDateTime end = LocalDateTime.now();
            job.setStatus("failed");
            job.setStage("failed");
            job.setProgress(100);
            job.setFailedStages(List.of("chunking"));
            job.setErrorMsg(trimError(e.getMessage()));
            job.setEndTime(end);
            job.setUpdatedAt(end);
        }
        documentParseJobRepository.save(job);
    }

    private DocumentParseChunkVO toChunkVO(KnowledgeChunk chunk) {
        DocumentParseChunkVO vo = new DocumentParseChunkVO();
        vo.setChunkId(String.valueOf(chunk.getId()));
        vo.setStableId(chunk.getStableHash());
        vo.setStableIdVersion(chunkingProperties.getStableIdVersion());
        vo.setStrategyVersion(chunk.getStrategyVersion());
        vo.setChunkIndex(chunk.getChunkIndex());
        vo.setContent(chunk.getChunkText());
        vo.setLength(chunk.getChunkLength());
        vo.setTitlePath(chunk.getTitlePath());
        vo.setPageStart(chunk.getPageStart());
        vo.setPageEnd(chunk.getPageEnd());
        vo.setAnchor(parseAnchor(chunk.getAnchorJson()));
        vo.setCreatedAt(chunk.getCreateTime() == null ? null : LocalDateTime.ofInstant(chunk.getCreateTime().toInstant(), java.time.ZoneId.systemDefault()));
        return vo;
    }

    private Map<String, Object> parseAnchor(String anchorJson) {
        if (!StringUtils.hasText(anchorJson)) {
            return Map.of();
        }
        try {
            return objectMapper.readValue(anchorJson, new TypeReference<Map<String, Object>>() {
            });
        } catch (Exception ex) {
            return Map.of();
        }
    }

    private CreateDocumentParseJobVO toCreateVO(DocumentParseJobEntity entity) {
        CreateDocumentParseJobVO vo = new CreateDocumentParseJobVO();
        vo.setJobId(entity.getJobId());
        vo.setStatus(entity.getStatus());
        vo.setCreatedAt(entity.getCreatedAt());
        return vo;
    }

    private DocumentParseJobEntity loadJob(String jobId) {
        return documentParseJobRepository.findByJobId(jobId)
                .orElseThrow(() -> new BizException(5620, "任务不存在"));
    }

    private Long parseFileId(String fileId) {
        try {
            return Long.parseLong(fileId);
        } catch (Exception ex) {
            throw new BizException(5610, "fileId不合法");
        }
    }

    private Long parseSinceChunkId(String sinceChunkId) {
        if (!StringUtils.hasText(sinceChunkId)) {
            return null;
        }
        try {
            return Long.parseLong(sinceChunkId);
        } catch (Exception ex) {
            throw new BizException(5610, "sinceChunkId不合法");
        }
    }

    private String resolveTriggerStatus(DocumentParseJobEntity entity) {
        if (!Boolean.TRUE.equals(entity.getTriggerRag())) {
            return "not_enabled";
        }
        Optional<RagTriggerOutboxEntity> outbox = ragTriggerOutboxRepository.findTopByJobIdOrderByCreatedAtDesc(entity.getJobId());
        if (outbox.isEmpty()) {
            return "pending";
        }
        return normalizeOutboxStatus(outbox.get().getStatus());
    }

    private String normalizeOutboxStatus(String status) {
        if (!StringUtils.hasText(status)) {
            return "pending";
        }
        String value = status.toLowerCase();
        if ("new".equals(value)) {
            return "pending";
        }
        if ("sending".equals(value)) {
            return "sending";
        }
        if ("sent".equals(value)) {
            return "sent";
        }
        if ("retrying".equals(value)) {
            return "retrying";
        }
        if ("dlq".equals(value)) {
            return "dlq";
        }
        return "pending";
    }

    private String buildJobId() {
        return "dpj_" + System.currentTimeMillis() + "_" + UUID.randomUUID().toString().replace("-", "").substring(0, 8);
    }

    private String trimError(String errorMsg) {
        if (!StringUtils.hasText(errorMsg)) {
            return "解析失败";
        }
        if (errorMsg.length() <= ERROR_MSG_MAX_LENGTH) {
            return errorMsg;
        }
        return errorMsg.substring(0, ERROR_MSG_MAX_LENGTH);
    }

    private String normalizeLower(String value) {
        if (!StringUtils.hasText(value)) {
            return "";
        }
        return value.trim().toLowerCase();
    }
}
