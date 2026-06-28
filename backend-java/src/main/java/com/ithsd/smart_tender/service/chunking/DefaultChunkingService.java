package com.ithsd.smart_tender.service.chunking;

import com.ithsd.smart_tender.service.extract.model.ParsedBlock;
import dev.langchain4j.data.embedding.Embedding;
import dev.langchain4j.data.segment.TextSegment;
import dev.langchain4j.model.embedding.EmbeddingModel;
import dev.langchain4j.model.output.Response;
import org.springframework.beans.factory.ObjectProvider;
import org.springframework.stereotype.Component;
import org.springframework.util.StringUtils;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.regex.Pattern;

@Component
public class DefaultChunkingService implements ChunkingService {
    private static final Pattern SENTENCE_PATTERN = Pattern.compile("(?<=[。！？；.!?;])\\s*|\\n+");
    private static final Pattern ARTICLE_MARKER_PATTERN = Pattern.compile(
            "(?:(?<=^)|(?<=[\\s\\n]))(?:第[一二三四五六七八九十百千万0-9]+条|\\d+\\.\\d+(?:\\.\\d+)*|\\d+、|\\([一二三四五六七八九十0-9]+\\)|（[一二三四五六七八九十0-9]+）)"
    );
    private static final Pattern PRIORITY_TABLE_FIELD_PATTERN = Pattern.compile("(金额|单价|合价|总价|预算|税费|证书号|证书编号|证书|有效期|到期|截止|发证|许可证|资质|信用代码)");

    private final ChunkingProperties properties;
    private final StableIdGenerator stableIdGenerator;
    private final EmbeddingModel embeddingModel;

    public DefaultChunkingService(
            ChunkingProperties properties,
            StableIdGenerator stableIdGenerator,
            ObjectProvider<EmbeddingModel> embeddingModelProvider
    ) {
        this.properties = properties;
        this.stableIdGenerator = stableIdGenerator;
        this.embeddingModel = embeddingModelProvider.getIfAvailable();
    }

    @Override
    public List<ChunkSlice> chunk(List<ParsedBlock> blocks) {
        if (blocks == null || blocks.isEmpty()) {
            return List.of();
        }
        List<Unit> units = buildUnits(blocks);
        if (units.isEmpty()) {
            return List.of();
        }
        String mode = normalizeMode(properties.getMode());
        List<Unit> windowUnits = units;
        if ("semantic".equals(mode) || "hybrid".equals(mode)) {
            windowUnits = mergeBySemantic(units);
        }
        List<ChunkSlice> slices = new ArrayList<>();
        int start = 0;
        while (start < windowUnits.size()) {
            ChunkWindow window = nextWindow(windowUnits, start);
            ChunkSlice slice = buildSlice(slices.size(), windowUnits, window.start, window.end);
            slices.add(slice);
            if (window.end >= windowUnits.size()) {
                break;
            }
            int nextStart = calculateNextStart(windowUnits, window.start, window.end, slice.getLength());
            if (nextStart <= start) {
                nextStart = window.end;
            }
            start = nextStart;
        }
        return slices;
    }

    private List<Unit> buildUnits(List<ParsedBlock> blocks) {
        List<Unit> units = new ArrayList<>();
        boolean evidenceSplit = !"paragraph".equalsIgnoreCase(properties.getSplitUnit());
        for (ParsedBlock block : blocks) {
            String blockType = normalizeText(block.getBlockType()).toLowerCase(Locale.ROOT);
            String titlePath = normalizeText(block.getTitlePath());
            String anchor = buildAnchor(block);
            if (isTableBlock(blockType)) {
                appendTableUnits(units, block, titlePath, anchor);
                continue;
            }
            String raw = normalizeText(block.getText());
            if (!StringUtils.hasText(raw)) {
                continue;
            }
            String text = raw;
            if (properties.isTitleInjection() && StringUtils.hasText(titlePath) && !raw.startsWith(titlePath)) {
                text = titlePath + "\n" + raw;
            }
            if (evidenceSplit) {
                List<String> evidenceUnits = splitEvidenceUnits(text);
                for (String evidenceUnit : evidenceUnits) {
                    if (shouldKeep(evidenceUnit)) {
                        units.add(new Unit(evidenceUnit, titlePath, block.getLogicalPage(), block.getLogicalPage(), anchor, anchor));
                    }
                }
            } else {
                if (shouldKeep(text)) {
                    units.add(new Unit(text, titlePath, block.getLogicalPage(), block.getLogicalPage(), anchor, anchor));
                }
            }
        }
        return units;
    }

    private List<String> splitEvidenceUnits(String text) {
        if (!StringUtils.hasText(text)) {
            return List.of();
        }
        List<String> units = new ArrayList<>();
        String[] sentenceUnits = SENTENCE_PATTERN.split(text);
        for (String sentenceUnit : sentenceUnits) {
            String normalized = normalizeText(sentenceUnit);
            if (!StringUtils.hasText(normalized)) {
                continue;
            }
            List<String> articleUnits = splitByArticleMarker(normalized);
            if (articleUnits.isEmpty()) {
                units.add(normalized);
            } else {
                units.addAll(articleUnits);
            }
        }
        return units;
    }

    private List<String> splitByArticleMarker(String text) {
        if (!StringUtils.hasText(text)) {
            return List.of();
        }
        List<String> parts = new ArrayList<>();
        java.util.regex.Matcher matcher = ARTICLE_MARKER_PATTERN.matcher(text);
        int lastStart = 0;
        while (matcher.find()) {
            int markerStart = matcher.start();
            if (markerStart <= lastStart) {
                continue;
            }
            String prev = normalizeText(text.substring(lastStart, markerStart));
            if (StringUtils.hasText(prev)) {
                parts.add(prev);
            }
            lastStart = markerStart;
        }
        String tail = normalizeText(text.substring(lastStart));
        if (StringUtils.hasText(tail)) {
            parts.add(tail);
        }
        return parts;
    }

    private void appendTableUnits(List<Unit> units, ParsedBlock block, String titlePath, String blockAnchor) {
        String tableText = normalizeTableText(block.getText());
        if (!StringUtils.hasText(tableText)) {
            return;
        }
        Integer page = block.getLogicalPage();
        String[] rows = splitTableRows(tableText);
        int added = 0;
        for (int i = 0; i < rows.length; i++) {
            String row = normalizeText(rows[i]);
            if (!shouldKeepTableRow(row)) {
                continue;
            }
            boolean priority = isPriorityTableRow(row);
            String rowText = row;
            if (priority) {
                rowText = "重点字段 " + row;
            }
            if (properties.isTitleInjection() && StringUtils.hasText(titlePath) && !rowText.startsWith(titlePath)) {
                rowText = titlePath + "\n" + rowText;
            }
            String rowAnchor = blockAnchor + "#r" + (i + 1);
            units.add(new Unit(rowText, titlePath, page, page, rowAnchor, rowAnchor));
            added++;
        }
        if (added == 0 && shouldKeep(tableText)) {
            String text = tableText;
            if (properties.isTitleInjection() && StringUtils.hasText(titlePath) && !text.startsWith(titlePath)) {
                text = titlePath + "\n" + text;
            }
            units.add(new Unit(text, titlePath, page, page, blockAnchor, blockAnchor));
        }
    }

    private List<Unit> mergeBySemantic(List<Unit> units) {
        if (units.size() <= 1) {
            return units;
        }
        int semanticMin = clamp(properties.getSemanticMinLength(), 120, Math.max(120, properties.getSemanticMaxLength()));
        int semanticMax = Math.max(semanticMin, properties.getSemanticMaxLength());
        double threshold = clampSimilarity(properties.getSemanticSimilarityThreshold());
        List<float[]> vectors = embedUnits(units);
        List<Unit> merged = new ArrayList<>();
        int segmentStart = 0;
        int segmentLength = units.get(0).text.length();
        for (int i = 1; i < units.size(); i++) {
            double similarity = sentenceSimilarity(units, vectors, i - 1, i);
            boolean weakBoundary = similarity < threshold;
            boolean exceedMax = segmentLength >= semanticMax;
            if (exceedMax || (weakBoundary && segmentLength >= semanticMin)) {
                merged.add(mergeRange(units, segmentStart, i));
                segmentStart = i;
                segmentLength = 0;
            }
            segmentLength += units.get(i).text.length() + 1;
        }
        if (segmentStart < units.size()) {
            merged.add(mergeRange(units, segmentStart, units.size()));
        }
        return merged.isEmpty() ? units : merged;
    }

    private Unit mergeRange(List<Unit> units, int start, int end) {
        List<Unit> range = units.subList(start, end);
        StringBuilder builder = new StringBuilder();
        String title = "";
        for (Unit unit : range) {
            if (builder.length() > 0) {
                builder.append('\n');
            }
            builder.append(unit.text);
            if (!StringUtils.hasText(title) && StringUtils.hasText(unit.titlePath)) {
                title = unit.titlePath;
            }
        }
        Unit first = range.get(0);
        Unit last = range.get(range.size() - 1);
        return new Unit(
                builder.toString(),
                title,
                first.pageStart,
                last.pageEnd,
                first.anchorStart,
                last.anchorEnd
        );
    }

    private List<float[]> embedUnits(List<Unit> units) {
        if (embeddingModel == null || units.isEmpty()) {
            return Collections.emptyList();
        }
        try {
            List<TextSegment> segments = new ArrayList<>(units.size());
            for (Unit unit : units) {
                segments.add(TextSegment.from(unit.text));
            }
            Response<List<Embedding>> response = embeddingModel.embedAll(segments);
            List<Embedding> embeddings = response == null ? null : response.content();
            if (embeddings == null || embeddings.size() != units.size()) {
                return Collections.emptyList();
            }
            List<float[]> vectors = new ArrayList<>(embeddings.size());
            for (Embedding embedding : embeddings) {
                vectors.add(embedding.vector());
            }
            return vectors;
        } catch (Exception ignored) {
            return Collections.emptyList();
        }
    }

    private double sentenceSimilarity(List<Unit> units, List<float[]> vectors, int left, int right) {
        if (!vectors.isEmpty() && left < vectors.size() && right < vectors.size()) {
            return cosineSimilarity(vectors.get(left), vectors.get(right));
        }
        return tokenJaccard(units.get(left).text, units.get(right).text);
    }

    private double cosineSimilarity(float[] left, float[] right) {
        if (left == null || right == null || left.length == 0 || left.length != right.length) {
            return 0d;
        }
        double dot = 0d;
        double leftNorm = 0d;
        double rightNorm = 0d;
        for (int i = 0; i < left.length; i++) {
            dot += left[i] * right[i];
            leftNorm += left[i] * left[i];
            rightNorm += right[i] * right[i];
        }
        if (leftNorm <= 0d || rightNorm <= 0d) {
            return 0d;
        }
        return dot / (Math.sqrt(leftNorm) * Math.sqrt(rightNorm));
    }

    private double tokenJaccard(String left, String right) {
        String[] leftTokens = splitSimilarityTokens(left);
        String[] rightTokens = splitSimilarityTokens(right);
        if (leftTokens.length == 0 || rightTokens.length == 0) {
            return 0d;
        }
        java.util.Set<String> leftSet = new java.util.HashSet<>();
        java.util.Set<String> rightSet = new java.util.HashSet<>();
        Collections.addAll(leftSet, leftTokens);
        Collections.addAll(rightSet, rightTokens);
        int intersect = 0;
        for (String token : leftSet) {
            if (rightSet.contains(token)) {
                intersect++;
            }
        }
        int union = leftSet.size() + rightSet.size() - intersect;
        return union == 0 ? 0d : (double) intersect / union;
    }

    private String[] splitSimilarityTokens(String text) {
        if (!StringUtils.hasText(text)) {
            return new String[0];
        }
        return normalizeText(text).toLowerCase(Locale.ROOT).split("[\\s,，。;；:：!！?？()（）\\[\\]【】]+");
    }

    private ChunkWindow nextWindow(List<Unit> units, int start) {
        int target = clamp(properties.getTargetLength(), properties.getMinLength(), properties.getMaxLength());
        int min = clamp(properties.getMinLength(), 1, properties.getMaxLength());
        int max = Math.max(min, properties.getMaxLength());
        int currentLength = 0;
        int end = start;
        while (end < units.size()) {
            int nextLength = currentLength == 0 ? units.get(end).text.length() : currentLength + 1 + units.get(end).text.length();
            if (currentLength > 0 && nextLength > max) {
                break;
            }
            currentLength = nextLength;
            end++;
            if (currentLength >= target) {
                break;
            }
        }
        if (end == start) {
            end = Math.min(start + 1, units.size());
        } else if (currentLength < min && end < units.size()) {
            while (end < units.size() && currentLength < min) {
                currentLength += 1 + units.get(end).text.length();
                end++;
            }
        }
        return new ChunkWindow(start, end);
    }

    private int calculateNextStart(List<Unit> units, int start, int end, int chunkLength) {
        if (end >= units.size()) {
            return end;
        }
        double overlapRatio = clampOverlap(properties.getOverlapRatio(), properties.getOverlapMinRatio(), properties.getOverlapMaxRatio());
        int overlapChars = Math.max(1, (int) Math.round(chunkLength * overlapRatio));
        int counted = 0;
        int cursor = end - 1;
        while (cursor > start) {
            counted += units.get(cursor).text.length();
            if (counted >= overlapChars) {
                return cursor;
            }
            cursor--;
        }
        return end;
    }

    private ChunkSlice buildSlice(int index, List<Unit> units, int start, int end) {
        List<Unit> window = units.subList(start, end);
        String content = joinWindow(window);
        String titlePath = firstNonBlank(window);
        int pageStart = minPage(window);
        int pageEnd = maxPage(window);
        String anchor = window.get(0).anchorStart + "->" + window.get(window.size() - 1).anchorEnd;
        ChunkSlice slice = new ChunkSlice();
        slice.setChunkIndex(index);
        slice.setContent(content);
        slice.setLength(content.length());
        slice.setTitlePath(titlePath);
        slice.setPageStart(pageStart);
        slice.setPageEnd(pageEnd);
        slice.setAnchor(anchor);
        slice.setStrategyVersion(properties.getStrategyVersion());
        slice.setStableIdVersion(properties.getStableIdVersion());
        slice.setStableId(stableIdGenerator.generate(content, titlePath, anchor));
        return slice;
    }

    private String joinWindow(List<Unit> units) {
        StringBuilder builder = new StringBuilder();
        for (Unit unit : units) {
            if (builder.length() > 0) {
                builder.append('\n');
            }
            builder.append(unit.text);
        }
        return builder.toString();
    }

    private String firstNonBlank(List<Unit> units) {
        for (Unit unit : units) {
            if (StringUtils.hasText(unit.titlePath)) {
                return unit.titlePath;
            }
        }
        return "";
    }

    private int minPage(List<Unit> units) {
        int min = Integer.MAX_VALUE;
        for (Unit unit : units) {
            if (unit.pageStart != null) {
                min = Math.min(min, unit.pageStart);
            }
        }
        return min == Integer.MAX_VALUE ? 1 : min;
    }

    private int maxPage(List<Unit> units) {
        int max = Integer.MIN_VALUE;
        for (Unit unit : units) {
            if (unit.pageEnd != null) {
                max = Math.max(max, unit.pageEnd);
            }
        }
        return max == Integer.MIN_VALUE ? 1 : max;
    }

    private boolean shouldKeep(String text) {
        if (!StringUtils.hasText(text)) {
            return false;
        }
        if (!properties.isDenoiseEnabled()) {
            return true;
        }
        String normalized = text.toLowerCase(Locale.ROOT).trim();
        if (normalized.length() <= 1) {
            return false;
        }
        if (normalized.matches("^[\\p{Punct}，。！？；：、\\s]+$")) {
            return false;
        }
        if (normalized.contains("免责声明") || normalized.contains("版权") || normalized.contains("copyright")) {
            return false;
        }
        if (normalized.matches(".*第\\s*\\d+\\s*页.*共\\s*\\d+\\s*页.*")) {
            return false;
        }
        return true;
    }

    private boolean shouldKeepTableRow(String row) {
        if (!StringUtils.hasText(row)) {
            return false;
        }
        if (isPriorityTableRow(row)) {
            return true;
        }
        return shouldKeep(row) && row.length() >= 4;
    }

    private boolean isPriorityTableRow(String row) {
        if (!StringUtils.hasText(row)) {
            return false;
        }
        return PRIORITY_TABLE_FIELD_PATTERN.matcher(row).find();
    }

    private String normalizeText(String text) {
        if (!StringUtils.hasText(text)) {
            return "";
        }
        return text.replace('\u00A0', ' ').replaceAll("\\s+", " ").trim();
    }

    private String normalizeTableText(String text) {
        if (!StringUtils.hasText(text)) {
            return "";
        }
        String normalized = text.replace('\u00A0', ' ').replace("\r\n", "\n").replace('\r', '\n');
        String[] lines = normalized.split("\n");
        StringBuilder builder = new StringBuilder();
        for (String line : lines) {
            String row = line == null ? "" : line.replaceAll("\\s+", " ").trim();
            if (!StringUtils.hasText(row)) {
                continue;
            }
            if (builder.length() > 0) {
                builder.append('\n');
            }
            builder.append(row);
        }
        return builder.toString();
    }

    private String[] splitTableRows(String tableText) {
        if (!StringUtils.hasText(tableText)) {
            return new String[0];
        }
        String[] byLine = tableText.split("\n+");
        if (byLine.length > 1) {
            return byLine;
        }
        return tableText.split("\\s*\\|\\s*");
    }

    private boolean isTableBlock(String blockType) {
        if (!StringUtils.hasText(blockType)) {
            return false;
        }
        return blockType.contains("table");
    }

    private String buildAnchor(ParsedBlock block) {
        String type = StringUtils.hasText(block.getAnchorType()) ? block.getAnchorType() : "paragraph";
        String id = StringUtils.hasText(block.getAnchorId()) ? block.getAnchorId() : String.valueOf(block.getBlockIndex());
        return type + ":" + id;
    }

    private int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }

    private double clampOverlap(double value, double min, double max) {
        return Math.max(min, Math.min(max, value));
    }

    private double clampSimilarity(double value) {
        if (Double.isNaN(value) || Double.isInfinite(value)) {
            return 0.72d;
        }
        return Math.max(0.45d, Math.min(0.95d, value));
    }

    private String normalizeMode(String mode) {
        if (!StringUtils.hasText(mode)) {
            return "hybrid";
        }
        String normalized = mode.trim().toLowerCase(Locale.ROOT);
        if ("semantic".equals(normalized) || "hybrid".equals(normalized) || "sliding".equals(normalized)) {
            return normalized;
        }
        return "hybrid";
    }

    private record Unit(
            String text,
            String titlePath,
            Integer pageStart,
            Integer pageEnd,
            String anchorStart,
            String anchorEnd
    ) {
    }

    private record ChunkWindow(int start, int end) {
    }
}
