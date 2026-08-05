package com.ithsd.smart_tender.service;

import com.ithsd.smart_tender.model.entity.KnowledgeChunk;
import java.io.IOException;
import java.util.List;

public interface KnowledgeChunkService {

    /**
     * 处理文件分块
     * @param fileId 文件ID
     * @param filePath 文件路径
     * @param namespace 命名空间(kb/tender)
     * @throws IOException  IOException
     */
    void processFileChunks(Long fileId, String filePath, String namespace) throws IOException;

    /**
     * 根据文件ID获取分块列表
     * @param fileId 文件ID
     * @return 分块列表
     */
    List<KnowledgeChunk> getChunksByFileId(Long fileId);

    /**
     * 删除文件的所有分块
     * @param fileId 文件ID
     */
    void deleteChunksByFileId(Long fileId);
}
