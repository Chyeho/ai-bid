package com.ithsd.smart_tender.service.impl;

import com.alibaba.fastjson.JSONObject;
import com.baomidou.mybatisplus.core.conditions.query.LambdaQueryWrapper;
import com.google.gson.Gson;
import com.google.gson.JsonObject;
import com.ithsd.smart_tender.mapper.KnowledgeChunkMapper;
import com.ithsd.smart_tender.pojo.entity.KnowledgeChunk;
import com.ithsd.smart_tender.service.chunking.ChunkSlice;
import com.ithsd.smart_tender.service.chunking.ChunkingService;
import com.ithsd.smart_tender.service.extract.DocParseProperties;
import com.ithsd.smart_tender.service.KnowledgeChunkService;
import com.ithsd.smart_tender.service.preview.DocumentPreviewService;
import com.ithsd.smart_tender.service.trigger.RagTriggerOutboxService;
import com.ithsd.smart_tender.service.trigger.RagTriggerProperties;
import dev.langchain4j.data.document.Document;
import dev.langchain4j.data.document.DocumentSplitter;
import dev.langchain4j.data.document.loader.FileSystemDocumentLoader;
import dev.langchain4j.data.document.parser.apache.pdfbox.ApachePdfBoxDocumentParser;
import dev.langchain4j.data.document.splitter.DocumentSplitters;
import dev.langchain4j.data.embedding.Embedding;
import dev.langchain4j.data.segment.TextSegment;
import dev.langchain4j.model.embedding.EmbeddingModel;
import dev.langchain4j.model.output.Response;
import io.milvus.v2.client.MilvusClientV2;
import io.milvus.v2.service.vector.request.DeleteReq;
import io.milvus.v2.service.vector.request.InsertReq;
import io.milvus.v2.service.vector.response.DeleteResp;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;
import org.springframework.util.StringUtils;
import org.apache.poi.xwpf.usermodel.IBodyElement;
import org.apache.poi.xwpf.usermodel.XWPFDocument;
import org.apache.poi.xwpf.usermodel.XWPFParagraph;
import org.apache.poi.xwpf.usermodel.XWPFRun;
import org.apache.poi.xwpf.usermodel.XWPFTable;
import org.openxmlformats.schemas.wordprocessingml.x2006.main.STBrType;
import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.text.PDFTextStripper;

import java.io.IOException;
import java.io.FileInputStream;
import java.nio.file.Paths;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Date;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

@Slf4j
@Service
public class KnowledgeChunkServiceImpl implements KnowledgeChunkService {
    private static final int LOGICAL_PAGE_SIZE = 1800;
    private static final Pattern PAGE_HINT_PATTERN = Pattern.compile("第\\s*(\\d+)\\s*页");
    private static final Pattern PARTY_A_PATTERN = Pattern.compile("(甲方\\s*[：:]\\s*)([^\\n]{2,120})");
    private static final Pattern PARTY_B_PATTERN = Pattern.compile("(乙方\\s*[：:]\\s*)([^\\n]{2,120})");
    private static final Pattern LEGAL_REP_PATTERN = Pattern.compile("(法定代表人\\s*[：:]\\s*)([^\\n，。；;：:]{2,30})");
    private static final Pattern ADDRESS_PATTERN = Pattern.compile("(地址\\s*[：:]\\s*)([^\\n]{4,180}?)(?=(?:\\s*(?:联系(?:电话|方式)|电话|邮箱|法定代表人|甲方|乙方)\\s*[：:]|\\n|$))");
    private static final Pattern CONTACT_PHONE_PATTERN = Pattern.compile("((?:联系(?:电话|方式)|电话)\\s*[：:]\\s*)([^\\n，。；;]{5,40})");
    private static final Pattern ID_CARD_PATTERN = Pattern.compile("(?<!\\d)(\\d{17}[\\dXx])(?!\\d)");
    private static final Pattern PHONE_PATTERN = Pattern.compile("(?<!\\d)(1[3-9]\\d{9})(?!\\d)");
    private static final Pattern EMAIL_PATTERN = Pattern.compile("([A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\\.[A-Za-z]{2,})");
    private static final Pattern USCC_PATTERN = Pattern.compile("(?<![A-Z0-9])([0-9A-Z]{18})(?![A-Z0-9])");
    private static final Pattern CERT_NO_PATTERN = Pattern.compile("(证书(?:编号|号)?\\s*[：:]\\s*)([A-Za-z0-9\\-_/]{4,64})");
    private static final Pattern ORG_PATTERN = Pattern.compile("([\\u4e00-\\u9fa5A-Za-z0-9（）()·\\-]{4,120}(?:有限公司|有限责任公司|股份有限公司|集团|研究院|医院|学校|大学))");
    @Autowired
    private KnowledgeChunkMapper knowledgeChunkMapper;
    @Autowired(required = false)
    private MilvusClientV2 milvusClient;
    @Autowired(required = false)
    private EmbeddingModel embeddingModel;
    @Autowired
    private ChunkingService chunkingService;
    @Autowired(required = false)
    private DocParseProperties docParseProperties;
    @Autowired(required = false)
    private RagTriggerOutboxService ragTriggerOutboxService;
    @Autowired
    private RagTriggerProperties ragTriggerProperties;
    @Autowired(required = false)
    private DocumentPreviewService documentPreviewService;

    @Value("${milvus.collection.name:knowledge_base}")
    private String collectionName;
    @Value("${audit.mask.enabled:true}")
    private boolean maskEnabled;

    private final Gson gson = new Gson();

    @Override
    @Transactional(rollbackFor = Exception.class)
    public void processFileChunks(Long fileId, String filePath, String namespace) throws IOException {
        if (docParseProperties != null && !docParseProperties.isEnabled()) {
            log.info("文档解析开关关闭，跳过处理：fileId={}, filePath={}", fileId, filePath);
            return;
        }
        log.info("开始处理文件分块：fileId={}, filePath={}", fileId, filePath);
        String fileExtension = filePath.substring(filePath.lastIndexOf(".") + 1).toLowerCase();
        List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> blocks;
        if ("docx".equals(fileExtension) || "doc".equals(fileExtension)) {
            try {
                if (documentPreviewService == null) {
                    throw new IOException("文档预览服务不可用");
                }
                Path pdfPath = documentPreviewService.ensurePdfPreviewFile(Paths.get(filePath));
                blocks = parsePdfBlocks(pdfPath.toString());
                fileExtension = "pdf";
            } catch (Exception ex) {
                log.warn("Word转PDF切片失败，回退原逻辑: fileId={}, path={}, err={}", fileId, filePath, ex.getMessage());
                if ("docx".equals(fileExtension)) {
                    blocks = parseDocxBlocks(filePath);
                } else {
                    Document document = FileSystemDocumentLoader.loadDocument(Paths.get(filePath));
                    List<Document> documents = List.of(document);
                    DocumentSplitter splitter = DocumentSplitters.recursive(500, 100);
                    List<TextSegment> segments = splitter.splitAll(documents);
                    blocks = buildBlocksByLogicalPage(segments);
                }
            }
        } else if ("pdf".equals(fileExtension)) {
            blocks = parsePdfBlocks(filePath);
        } else {
            Document document = FileSystemDocumentLoader.loadDocument(Paths.get(filePath));
            List<Document> documents = List.of(document);
            DocumentSplitter splitter = DocumentSplitters.recursive(500, 100);
            List<TextSegment> segments = splitter.splitAll(documents);
            blocks = buildBlocksByLogicalPage(segments);
        }
        List<ChunkSlice> slices = chunkingService.chunk(blocks);
        log.info("文件切片完成，共 {} 个切片", slices.size());
        List<KnowledgeChunk> incomingChunks = buildKnowledgeChunks(fileId, slices);
        applyIncrementalChanges(fileId, incomingChunks);
        enqueueRagTrigger(fileId, incomingChunks, namespace);
        storeVectors(fileId, filePath, fileExtension, incomingChunks, namespace);
    }

    private void enqueueRagTrigger(Long fileId, List<KnowledgeChunk> incomingChunks, String namespace) {
        if (!ragTriggerProperties.isEnabled() || ragTriggerOutboxService == null) {
            return;
        }
        String strategyVersion = incomingChunks.isEmpty() ? "chunk-v1" : incomingChunks.get(0).getStrategyVersion();
        String payloadHash = payloadHash(fileId, strategyVersion, incomingChunks);
        String jobId = "file-" + namespace + "-" + fileId;
        ragTriggerOutboxService.enqueue(fileId, incomingChunks.size(), strategyVersion, payloadHash, jobId);
    }

    void applyIncrementalChanges(Long fileId, List<KnowledgeChunk> incomingChunks) {
        LambdaQueryWrapper<KnowledgeChunk> queryWrapper = new LambdaQueryWrapper<>();
        queryWrapper.eq(KnowledgeChunk::getFileId, fileId);
        List<KnowledgeChunk> existing = knowledgeChunkMapper.selectList(queryWrapper);
        Map<String, KnowledgeChunk> existingMap = existing.stream()
                .collect(Collectors.toMap(this::chunkKey, chunk -> chunk, (a, b) -> a));
        Map<String, KnowledgeChunk> incomingMap = incomingChunks.stream()
                .collect(Collectors.toMap(this::chunkKey, chunk -> chunk, (a, b) -> a));

        for (KnowledgeChunk incoming : incomingChunks) {
            KnowledgeChunk existed = existingMap.get(chunkKey(incoming));
            if (existed == null) {
                knowledgeChunkMapper.insert(incoming);
            } else {
                incoming.setId(existed.getId());
                incoming.setCreateTime(existed.getCreateTime() == null ? new Date() : existed.getCreateTime());
                knowledgeChunkMapper.updateById(incoming);
            }
        }

        for (KnowledgeChunk chunk : existing) {
            if (!incomingMap.containsKey(chunkKey(chunk))) {
                knowledgeChunkMapper.deleteById(chunk.getId());
            }
        }
    }

    private List<KnowledgeChunk> buildKnowledgeChunks(Long fileId, List<ChunkSlice> slices) {
        List<KnowledgeChunk> result = new ArrayList<>();
        for (ChunkSlice slice : slices) {
            String maskedContent = maskChunkText(slice.getContent());
            KnowledgeChunk chunk = new KnowledgeChunk();
            chunk.setFileId(fileId);
            chunk.setChunkIndex(slice.getChunkIndex());
            chunk.setChunkText(maskedContent);
            chunk.setChunkLength(maskedContent.length());
            chunk.setStableHash(slice.getStableId());
            chunk.setStrategyVersion(slice.getStrategyVersion());
            JSONObject anchorJson = new JSONObject();
            anchorJson.put("anchor", slice.getAnchor());
            chunk.setAnchorJson(anchorJson.toJSONString());
            String maskedTitlePath = maskChunkText(slice.getTitlePath());
            chunk.setTitlePath(maskedTitlePath);
            chunk.setPageStart(slice.getPageStart());
            chunk.setPageEnd(slice.getPageEnd());
            chunk.setPageNumber(slice.getPageStart());
            chunk.setSectionName(maskedTitlePath);
            chunk.setCreateTime(new Date());
            result.add(chunk);
        }
        return result;
    }

    private String maskChunkText(String text) {
        if (!maskEnabled || !StringUtils.hasText(text)) {
            return text;
        }
        String masked = text;
        masked = maskByPattern(masked, PARTY_A_PATTERN, "$1PARTY_A_ENTITY");
        masked = maskByPattern(masked, PARTY_B_PATTERN, "$1PARTY_B_ENTITY");
        masked = maskByPattern(masked, LEGAL_REP_PATTERN, "$1LEGAL_REP");
        masked = maskByPattern(masked, ADDRESS_PATTERN, "$1ADDRESS");
        masked = maskByPattern(masked, CONTACT_PHONE_PATTERN, "$1CONTACT_PHONE");
        masked = maskByPattern(masked, CERT_NO_PATTERN, "$1CERT_NO");
        masked = maskByPattern(masked, ID_CARD_PATTERN, "ID_CARD");
        masked = maskByPattern(masked, PHONE_PATTERN, "PHONE");
        masked = maskByPattern(masked, EMAIL_PATTERN, "EMAIL");
        masked = maskByPattern(masked, USCC_PATTERN, "USCC_CODE");
        masked = maskOrganization(masked);
        return masked;
    }

    private String maskByPattern(String text, Pattern pattern, String replacement) {
        Matcher matcher = pattern.matcher(text);
        return matcher.replaceAll(replacement);
    }

    private String maskOrganization(String text) {
        Matcher matcher = ORG_PATTERN.matcher(text);
        StringBuffer buffer = new StringBuffer();
        while (matcher.find()) {
            String value = matcher.group(1);
            if ("PARTY_A_ENTITY".equals(value)
                    || "PARTY_B_ENTITY".equals(value)
                    || value.contains("LEGAL_REP")
                    || value.contains("CONTACT_PHONE")
                    || value.contains("ADDRESS")
                    || value.contains("USCC_CODE")
                    || value.contains("ID_CARD")
                    || value.contains("CERT_NO")) {
                matcher.appendReplacement(buffer, Matcher.quoteReplacement(value));
                continue;
            }
            matcher.appendReplacement(buffer, "ORG_ENTITY");
        }
        matcher.appendTail(buffer);
        return buffer.toString();
    }

    private List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> buildBlocksByLogicalPage(List<TextSegment> segments) {
        List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> blocks = new ArrayList<>();
        int logicalPage = 1;
        int currentPageChars = 0;
        for (int i = 0; i < segments.size(); i++) {
            String segmentText = segments.get(i).text();
            int length = StringUtils.hasText(segmentText) ? segmentText.length() : 1;
            if (currentPageChars > 0 && currentPageChars + length > LOGICAL_PAGE_SIZE) {
                logicalPage++;
                currentPageChars = 0;
            }
            com.ithsd.smart_tender.service.extract.model.ParsedBlock block = new com.ithsd.smart_tender.service.extract.model.ParsedBlock();
            block.setBlockIndex(i);
            block.setBlockType("paragraph");
            block.setText(segmentText);
            block.setLogicalPage(logicalPage);
            block.setAnchorType("paragraph");
            block.setAnchorId("p-" + (i + 1));
            blocks.add(block);
            currentPageChars += length;
        }
        return blocks;
    }

    private List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> parseDocxBlocks(String filePath) throws IOException {
        List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> blocks = new ArrayList<>();
        int page = 1;
        int paragraphIndex = 0;
        int tableIndex = 0;
        boolean hasExplicitPageSignal = false;
        try (FileInputStream in = new FileInputStream(filePath); XWPFDocument document = new XWPFDocument(in)) {
            for (IBodyElement element : document.getBodyElements()) {
                if (element instanceof XWPFParagraph paragraph) {
                    paragraphIndex++;
                    int breaks = countPageBreaks(paragraph);
                    String text = normalizeText(paragraph.getText());
                    Integer hintedPage = parsePageHint(text);
                    if (hintedPage != null && hintedPage > 0) {
                        page = hintedPage;
                        hasExplicitPageSignal = true;
                    }
                    if (StringUtils.hasText(text)) {
                        com.ithsd.smart_tender.service.extract.model.ParsedBlock block = new com.ithsd.smart_tender.service.extract.model.ParsedBlock();
                        block.setBlockIndex(blocks.size());
                        block.setBlockType("paragraph");
                        block.setText(text);
                        block.setLogicalPage(page);
                        block.setAnchorType("paragraph");
                        block.setAnchorId("p-" + paragraphIndex);
                        blocks.add(block);
                    }
                    if (breaks > 0) {
                        page += breaks;
                        hasExplicitPageSignal = true;
                    }
                } else if (element instanceof XWPFTable table) {
                    tableIndex++;
                    String text = normalizeText(table.getText());
                    if (!StringUtils.hasText(text)) {
                        text = "[TABLE]";
                    }
                    com.ithsd.smart_tender.service.extract.model.ParsedBlock block = new com.ithsd.smart_tender.service.extract.model.ParsedBlock();
                    block.setBlockIndex(blocks.size());
                    block.setBlockType("table");
                    block.setText(text);
                    block.setLogicalPage(page);
                    block.setAnchorType("table");
                    block.setAnchorId("t-" + tableIndex);
                    blocks.add(block);
                }
            }
        }
        if (!hasExplicitPageSignal) {
            return buildBlocksByLogicalPage(
                    blocks.stream()
                            .map(b -> TextSegment.from(StringUtils.hasText(b.getText()) ? b.getText() : ""))
                            .collect(Collectors.toList())
            );
        }
        return blocks;
    }

    private List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> parsePdfBlocks(String filePath) throws IOException {
        List<com.ithsd.smart_tender.service.extract.model.ParsedBlock> blocks = new ArrayList<>();
        try (PDDocument document = PDDocument.load(new java.io.File(filePath))) {
            PDFTextStripper stripper = new PDFTextStripper();
            int totalPages = document.getNumberOfPages();
            for (int page = 1; page <= totalPages; page++) {
                stripper.setStartPage(page);
                stripper.setEndPage(page);
                String text = normalizeText(stripper.getText(document));
                if (!StringUtils.hasText(text)) {
                    continue;
                }
                com.ithsd.smart_tender.service.extract.model.ParsedBlock block = new com.ithsd.smart_tender.service.extract.model.ParsedBlock();
                block.setBlockIndex(blocks.size());
                block.setBlockType("page");
                block.setText(text);
                block.setLogicalPage(page);
                block.setAnchorType("page");
                block.setAnchorId("pdf-p-" + page);
                blocks.add(block);
            }
        } catch (Exception ex) {
            log.warn("parsePdfBlocks失败，回退到逻辑分页切分: {}", ex.getMessage());
            ApachePdfBoxDocumentParser parser = new ApachePdfBoxDocumentParser();
            Document document;
            try (java.io.FileInputStream fis = new java.io.FileInputStream(filePath)) {
                document = parser.parse(fis);
            }
            List<Document> documents = List.of(document);
            DocumentSplitter splitter = DocumentSplitters.recursive(500, 100);
            List<TextSegment> segments = splitter.splitAll(documents);
            blocks = buildBlocksByLogicalPage(segments);
        }
        return blocks;
    }

    private int countPageBreaks(XWPFParagraph paragraph) {
        int count = 0;
        if (paragraph == null) {
            return 0;
        }
        List<XWPFRun> runs = paragraph.getRuns();
        if (runs == null || runs.isEmpty()) {
            return 0;
        }
        for (XWPFRun run : runs) {
            String text = run.text();
            if (text != null) {
                for (char ch : text.toCharArray()) {
                    if (ch == '\f') {
                        count++;
                    }
                }
            }
            if (run.getCTR() != null && run.getCTR().sizeOfBrArray() > 0) {
                for (int i = 0; i < run.getCTR().sizeOfBrArray(); i++) {
                    if (run.getCTR().getBrArray(i).getType() == STBrType.PAGE) {
                        count++;
                    }
                }
            }
        }
        return count;
    }

    private Integer parsePageHint(String text) {
        if (!StringUtils.hasText(text)) {
            return null;
        }
        Matcher matcher = PAGE_HINT_PATTERN.matcher(text);
        if (!matcher.find()) {
            return null;
        }
        try {
            return Integer.parseInt(matcher.group(1));
        } catch (Exception ignored) {
            return null;
        }
    }

    private String normalizeText(String value) {
        if (!StringUtils.hasText(value)) {
            return "";
        }
        return value.replace('\u00A0', ' ').replaceAll("\\s+", " ").trim();
    }

    private void storeVectors(Long fileId, String filePath, String fileExtension, List<KnowledgeChunk> chunks, String namespace) {
        if (chunks.isEmpty() || embeddingModel == null || milvusClient == null) {
            return;
        }
        String targetCollection = "tender".equalsIgnoreCase(namespace) ? "tender_rag" : "knowledge_base";
        List<TextSegment> segments = new ArrayList<>();
        for (KnowledgeChunk chunk : chunks) {
            if (StringUtils.hasText(chunk.getChunkText())) {
                segments.add(TextSegment.from(chunk.getChunkText()));
            }
        }
        if (segments.isEmpty()) {
            return;
        }
        try {
            Response<List<Embedding>> response = embeddingModel.embedAll(segments);
            List<Embedding> embeddings = response.content();
            List<JsonObject> data = new ArrayList<>();
            for (int i = 0; i < embeddings.size(); i++) {
                KnowledgeChunk chunk = chunks.get(i);
                JsonObject row = new JsonObject();
                com.google.gson.JsonArray vectorJsonArray = new com.google.gson.JsonArray();
                for (float value : embeddings.get(i).vector()) {
                    vectorJsonArray.add(value);
                }
                row.add("embedding", vectorJsonArray);
                row.addProperty("text", chunk.getChunkText());
                row.addProperty("file_id", fileId);
                row.addProperty("chunk_index", chunk.getChunkIndex());
                Map<String, Object> metadata = new HashMap<>();
                metadata.put("file_path", filePath);
                metadata.put("file_extension", fileExtension);
                metadata.put("stable_hash", chunk.getStableHash());
                metadata.put("strategy_version", chunk.getStrategyVersion());
                JsonObject metadataJson = gson.toJsonTree(metadata).getAsJsonObject();
                row.add("metadata", metadataJson);
                data.add(row);
            }
            if (!data.isEmpty()) {
                InsertReq insertReq = InsertReq.builder().collectionName(targetCollection).data(data).build();
                milvusClient.insert(insertReq);
            }
        } catch (Exception e) {
            log.error("向量存储失败：{}", e.getMessage(), e);
        }
    }

    private String chunkKey(KnowledgeChunk chunk) {
        String hash = chunk.getStableHash();
        if (!StringUtils.hasText(hash)) {
            hash = "idx-" + chunk.getChunkIndex();
        }
        String version = chunk.getStrategyVersion() == null ? "" : chunk.getStrategyVersion();
        return hash + "|" + version;
    }

    private String payloadHash(Long fileId, String strategyVersion, List<KnowledgeChunk> chunks) {
        String joinedHashes = chunks.stream()
                .map(chunk -> StringUtils.hasText(chunk.getStableHash()) ? chunk.getStableHash() : String.valueOf(chunk.getChunkIndex()))
                .sorted()
                .collect(Collectors.joining(","));
        String raw = fileId + "|" + strategyVersion + "|" + chunks.size() + "|" + joinedHashes;
        try {
            MessageDigest digest = MessageDigest.getInstance("SHA-256");
            byte[] bytes = digest.digest(raw.getBytes(StandardCharsets.UTF_8));
            StringBuilder builder = new StringBuilder();
            for (byte value : bytes) {
                builder.append(String.format("%02x", value));
            }
            return builder.toString();
        } catch (NoSuchAlgorithmException ex) {
            throw new IllegalStateException("SHA-256_NOT_SUPPORTED");
        }
    }

    @Override
    public List<KnowledgeChunk> getChunksByFileId(Long fileId) {
        LambdaQueryWrapper<KnowledgeChunk> queryWrapper = new LambdaQueryWrapper<>();
        queryWrapper.eq(KnowledgeChunk::getFileId, fileId);
        queryWrapper.orderByAsc(KnowledgeChunk::getChunkIndex);
        return knowledgeChunkMapper.selectList(queryWrapper);
    }

    @Override
    @Transactional(rollbackFor = Exception.class)
    public void deleteChunksByFileId(Long fileId) {
        log.info("开始删除文件分块：fileId={}", fileId);
        try {
            LambdaQueryWrapper<KnowledgeChunk> queryWrapper = new LambdaQueryWrapper<>();
            queryWrapper.eq(KnowledgeChunk::getFileId, fileId);
            List<KnowledgeChunk> chunks = knowledgeChunkMapper.selectList(queryWrapper);
            if (chunks.isEmpty()) {
                log.info("没有找到需要删除的分块数据：fileId={}", fileId);
                return;
            }
            log.info("找到 {} 条分块记录准备删除", chunks.size());
            try {
            if (milvusClient != null) {
                DeleteReq deleteReq = DeleteReq.builder()
                        .collectionName(collectionName)
                        .filter("file_id == " + fileId)
                        .build();
                DeleteResp deleteResp = milvusClient.delete(deleteReq);
                log.info("Milvus 向量删除成功，删除数量：{}", deleteResp.getDeleteCnt());
            } else {
                log.info("Milvus 已禁用，跳过向量删除");
            }
        } catch (Exception e) {
                log.error("Milvus 向量删除失败：{}", e.getMessage(), e);
            }
            knowledgeChunkMapper.delete(queryWrapper);
            log.info("数据库分块记录删除完成");
        } catch (Exception e) {
            log.error("删除文件分块失败：{}", e.getMessage(), e);
            throw new RuntimeException("删除文件分块失败：" + e.getMessage(), e);
        }
    }
}
