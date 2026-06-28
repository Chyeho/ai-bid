package com.ithsd.smart_tender.service.extract;

import com.ithsd.smart_tender.exception.BizException;
import com.ithsd.smart_tender.mapper.TenderMapper;
import com.ithsd.smart_tender.pojo.entity.Tender;
import com.ithsd.smart_tender.service.chunking.ChunkingService;
import com.ithsd.smart_tender.service.extract.model.ParsedBlock;
import com.ithsd.smart_tender.service.extract.model.ParsedDocument;
import org.apache.poi.xwpf.usermodel.IBodyElement;
import org.apache.poi.xwpf.usermodel.XWPFDocument;
import org.apache.poi.xwpf.usermodel.XWPFParagraph;
import org.apache.poi.xwpf.usermodel.XWPFRun;
import org.apache.poi.xwpf.usermodel.XWPFTable;
import org.apache.poi.xwpf.usermodel.XWPFTableCell;
import org.apache.poi.xwpf.usermodel.XWPFTableRow;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

@Component("wordDocumentExtractService")
public class WordExtractService implements DocumentExtractService {
    private static final int LOGICAL_PAGE_SIZE = 1800;
    private final TenderMapper tenderMapper;
    private final ChunkingService chunkingService;

    public WordExtractService(TenderMapper tenderMapper, ChunkingService chunkingService) {
        this.tenderMapper = tenderMapper;
        this.chunkingService = chunkingService;
    }

    @Override
    public ExtractedDocument extract(Long bidId) {
        Tender tender = tenderMapper.selectById(bidId);
        if (tender == null || !StringUtils.hasText(tender.getFilePath())) {
            throw new BizException(5612, "DOC_PARSE_FILE_NOT_FOUND");
        }
        File file = new File(tender.getFilePath());
        if (!file.exists() || !file.isFile()) {
            throw new BizException(5612, "DOC_PARSE_FILE_NOT_FOUND");
        }
        String extension = resolveExtension(tender, file);
        try {
            if ("docx".equals(extension)) {
                ParsedDocument parsed = parseDocx(file, bidId);
                return buildExtractedDocument(bidId, "word_docx", false, null, parsed);
            }
            if ("doc".equals(extension)) {
                ParsedDocument parsed = parseDoc(file, bidId);
                return buildExtractedDocument(bidId, "word_doc", true, "DOC_PARSE_DOC_DEGRADED", parsed);
            }
            throw new BizException(5611, "DOC_PARSE_UNSUPPORTED_FILE_TYPE");
        } catch (IOException ex) {
            throw new BizException(5632, "DOC_PARSE_ENGINE_FAILED");
        }
    }

    private String resolveExtension(Tender tender, File file) {
        String fileName = StringUtils.hasText(tender.getFileName()) ? tender.getFileName() : file.getName();
        String lower = fileName.toLowerCase();
        if (lower.endsWith(".docx")) {
            return "docx";
        }
        if (lower.endsWith(".doc")) {
            return "doc";
        }
        return "";
    }

    private ParsedDocument parseDocx(File file, Long bidId) throws IOException {
        ParsedDocument parsedDocument = new ParsedDocument();
        parsedDocument.setBidId(bidId);
        parsedDocument.setFileType("docx");
        List<ParsedBlock> blocks = new ArrayList<>();
        List<String> titleStack = new ArrayList<>();
        int paragraphIndex = 0;
        int tableIndex = 0;
        int imageIndex = 0;
        try (FileInputStream in = new FileInputStream(file); XWPFDocument document = new XWPFDocument(in)) {
            List<IBodyElement> elements = document.getBodyElements();
            for (IBodyElement element : elements) {
                if (element instanceof XWPFParagraph paragraph) {
                    paragraphIndex++;
                    String text = normalize(paragraph.getText());
                    int headingLevel = parseHeadingLevel(paragraph);
                    if (StringUtils.hasText(text)) {
                        ParsedBlock block = new ParsedBlock();
                        block.setBlockIndex(blocks.size());
                        block.setBlockType(headingLevel > 0 ? "heading" : "paragraph");
                        block.setText(text);
                        block.setTitleLevel(headingLevel > 0 ? headingLevel : null);
                        block.setAnchorType("paragraph");
                        block.setAnchorId("p-" + paragraphIndex);
                        if (headingLevel > 0) {
                            ensureTitleStackSize(titleStack, headingLevel);
                            titleStack.set(headingLevel - 1, text);
                        }
                        block.setTitlePath(buildTitlePath(titleStack));
                        blocks.add(block);
                    }
                    for (XWPFRun run : paragraph.getRuns()) {
                        int embeddedPictures = run.getEmbeddedPictures().size();
                        for (int i = 0; i < embeddedPictures; i++) {
                            imageIndex++;
                            ParsedBlock imageBlock = new ParsedBlock();
                            imageBlock.setBlockIndex(blocks.size());
                            imageBlock.setBlockType("image");
                            imageBlock.setText("[IMAGE]");
                            imageBlock.setAnchorType("image");
                            imageBlock.setAnchorId("img-" + imageIndex);
                            imageBlock.setTitlePath(buildTitlePath(titleStack));
                            blocks.add(imageBlock);
                        }
                    }
                }
                if (element instanceof XWPFTable table) {
                    tableIndex++;
                    String text = normalize(extractTableText(table));
                    if (!StringUtils.hasText(text)) {
                        text = "[TABLE]";
                    }
                    ParsedBlock tableBlock = new ParsedBlock();
                    tableBlock.setBlockIndex(blocks.size());
                    tableBlock.setBlockType("table");
                    tableBlock.setText(text);
                    tableBlock.setAnchorType("table");
                    tableBlock.setAnchorId("t-" + tableIndex);
                    tableBlock.setTitlePath(buildTitlePath(titleStack));
                    blocks.add(tableBlock);
                }
            }
        }
        assignLogicalPage(blocks);
        parsedDocument.setBlocks(blocks);
        return parsedDocument;
    }

    private ParsedDocument parseDoc(File file, Long bidId) throws IOException {
        ParsedDocument parsedDocument = new ParsedDocument();
        parsedDocument.setBidId(bidId);
        parsedDocument.setFileType("doc");
        List<ParsedBlock> blocks = new ArrayList<>();
        ParsedBlock block = new ParsedBlock();
        block.setBlockIndex(0);
        block.setBlockType("paragraph");
        block.setText("legacy_doc:" + file.getName());
        block.setAnchorType("paragraph");
        block.setAnchorId("p-1");
        blocks.add(block);
        assignLogicalPage(blocks);
        parsedDocument.setBlocks(blocks);
        return parsedDocument;
    }

    private ExtractedDocument buildExtractedDocument(Long bidId, String source, boolean degraded, String message, ParsedDocument parsedDocument) {
        ExtractedDocument extractedDocument = new ExtractedDocument();
        extractedDocument.setBidId(bidId);
        extractedDocument.setSource(source);
        extractedDocument.setDegraded(degraded);
        extractedDocument.setErrorMessage(message);
        extractedDocument.setParsedBlocks(parsedDocument.getBlocks());
        extractedDocument.setChunks(chunkingService.chunk(parsedDocument.getBlocks()));
        extractedDocument.setContent(joinContent(parsedDocument.getBlocks()));
        return extractedDocument;
    }

    private int parseHeadingLevel(XWPFParagraph paragraph) {
        String style = paragraph.getStyle();
        if (!StringUtils.hasText(style)) {
            return 0;
        }
        String normalized = style.toLowerCase();
        if (normalized.startsWith("heading")) {
            String suffix = normalized.substring("heading".length());
            try {
                return Integer.parseInt(suffix);
            } catch (NumberFormatException ex) {
                return 1;
            }
        }
        return 0;
    }

    private void ensureTitleStackSize(List<String> titleStack, int headingLevel) {
        while (titleStack.size() < headingLevel) {
            titleStack.add("");
        }
        while (titleStack.size() > headingLevel) {
            titleStack.remove(titleStack.size() - 1);
        }
    }

    private String buildTitlePath(List<String> titleStack) {
        List<String> values = new ArrayList<>();
        for (String title : titleStack) {
            if (StringUtils.hasText(title)) {
                values.add(title);
            }
        }
        return String.join(" > ", values);
    }

    private void assignLogicalPage(List<ParsedBlock> blocks) {
        int page = 1;
        int count = 0;
        for (ParsedBlock block : blocks) {
            int length = StringUtils.hasText(block.getText()) ? block.getText().length() : 1;
            if (count > 0 && count + length > LOGICAL_PAGE_SIZE) {
                page++;
                count = 0;
            }
            block.setLogicalPage(page);
            count += length;
        }
    }

    private String joinContent(List<ParsedBlock> blocks) {
        StringBuilder builder = new StringBuilder();
        for (ParsedBlock block : blocks) {
            if (!StringUtils.hasText(block.getText())) {
                continue;
            }
            if (builder.length() > 0) {
                builder.append('\n');
            }
            builder.append(block.getText());
        }
        return builder.toString();
    }

    private String normalize(String value) {
        if (!StringUtils.hasText(value)) {
            return "";
        }
        return value.replace('\u00A0', ' ').replaceAll("\\s+", " ").trim();
    }

    private String extractTableText(XWPFTable table) {
        if (table == null) {
            return "";
        }
        StringBuilder builder = new StringBuilder();
        List<XWPFTableRow> rows = table.getRows();
        if (rows == null || rows.isEmpty()) {
            return "";
        }
        for (XWPFTableRow row : rows) {
            if (row == null) {
                continue;
            }
            List<XWPFTableCell> cells = row.getTableCells();
            if (cells == null || cells.isEmpty()) {
                continue;
            }
            List<String> values = new ArrayList<>();
            for (XWPFTableCell cell : cells) {
                String cellText = normalize(cell == null ? "" : cell.getText());
                values.add(cellText);
            }
            if (builder.length() > 0) {
                builder.append('\n');
            }
            builder.append(String.join(" | ", values));
        }
        return builder.toString();
    }
}
