package com.ithsd.smart_tender.service.impl;

import com.ithsd.smart_tender.model.entity.KnowledgeChunk;
import com.ithsd.smart_tender.service.KnowledgeChunkService;
import lombok.extern.slf4j.Slf4j;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.io.IOException;
import java.util.Collections;
import java.util.List;

@Slf4j
@Service
public class KnowledgeChunkServiceImpl implements KnowledgeChunkService {

    @Override
    @Transactional(rollbackFor = Exception.class)
    public void processFileChunks(Long fileId, String filePath, String namespace) throws IOException {
        log.info("Java端切片已关闭，跳过：fileId={}, filePath={}", fileId, filePath);
    }

    @Override
    public List<KnowledgeChunk> getChunksByFileId(Long fileId) {
        return Collections.emptyList();
    }

    @Override
    @Transactional(rollbackFor = Exception.class)
    public void deleteChunksByFileId(Long fileId) {
        log.info("Java端切片已关闭，跳过删除：fileId={}", fileId);
    }
}
