package com.ithsd.smart_tender.common.util;

import org.apache.pdfbox.pdmodel.PDDocument;
import org.apache.pdfbox.pdmodel.PDPage;
import org.apache.pdfbox.pdmodel.common.PDRectangle;
import org.apache.pdfbox.pdmodel.font.PDFont;
import org.apache.pdfbox.pdmodel.font.PDType0Font;
import org.apache.pdfbox.pdmodel.font.PDType1Font;
import org.apache.pdfbox.pdmodel.PDPageContentStream;
import org.apache.poi.xwpf.usermodel.XWPFDocument;
import org.apache.poi.xwpf.usermodel.XWPFParagraph;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class DocxToPdfConverter {

    private DocxToPdfConverter() {
    }

    public static void convert(Path docxPath, Path pdfPath) throws IOException {
        try (InputStream in = Files.newInputStream(docxPath);
             XWPFDocument document = new XWPFDocument(in);
             PDDocument pdf = new PDDocument()) {

            PDFont font = loadFont(pdf);
            float fontSize = 11f;
            float leading = 1.5f * fontSize;

            PDPage page = new PDPage(PDRectangle.A4);
            pdf.addPage(page);
            PDPageContentStream contentStream = new PDPageContentStream(pdf, page);

            PDRectangle mediaBox = page.getMediaBox();
            float margin = 72;
            float width = mediaBox.getWidth() - 2 * margin;
            float startX = margin;
            float startY = mediaBox.getHeight() - margin;
            float currentY = startY;

            contentStream.beginText();
            contentStream.setFont(font, fontSize);
            contentStream.newLineAtOffset(startX, startY);

            for (XWPFParagraph paragraph : document.getParagraphs()) {
                String text = paragraph.getText();
                if (text == null || text.trim().isEmpty()) {
                    continue;
                }
                List<String> lines = wrapText(text, font, fontSize, width);
                for (String line : lines) {
                    contentStream.showText(line);
                    contentStream.newLineAtOffset(0, -leading);
                    currentY -= leading;
                    if (currentY <= margin) {
                        contentStream.endText();
                        contentStream.close();

                        page = new PDPage(PDRectangle.A4);
                        pdf.addPage(page);
                        contentStream = new PDPageContentStream(pdf, page);
                        mediaBox = page.getMediaBox();
                        startY = mediaBox.getHeight() - margin;
                        currentY = startY;

                        contentStream.beginText();
                        contentStream.setFont(font, fontSize);
                        contentStream.newLineAtOffset(startX, startY);
                    }
                }
                contentStream.newLineAtOffset(0, -leading);
                currentY -= leading;
            }

            contentStream.endText();
            contentStream.close();

            Files.createDirectories(pdfPath.getParent());
            pdf.save(pdfPath.toFile());
        }
    }

    private static PDFont loadFont(PDDocument pdf) throws IOException {
        Path windowsFont = Path.of("C:\\Windows\\Fonts\\msyh.ttc");
        if (Files.exists(windowsFont)) {
            return PDType0Font.load(pdf, Files.newInputStream(windowsFont), true);
        }
        return PDType1Font.HELVETICA;
    }

    private static List<String> wrapText(String text, PDFont font, float fontSize, float width) throws IOException {
        List<String> lines = new ArrayList<>();
        StringBuilder currentLine = new StringBuilder();
        for (int i = 0; i < text.length(); i++) {
            char c = text.charAt(i);
            currentLine.append(c);
            String candidate = currentLine.toString();
            float size = font.getStringWidth(candidate) / 1000f * fontSize;
            if (size > width) {
                if (currentLine.length() > 1) {
                    currentLine.setLength(currentLine.length() - 1);
                    lines.add(currentLine.toString());
                    currentLine.setLength(0);
                    currentLine.append(c);
                } else {
                    lines.add(currentLine.toString());
                    currentLine.setLength(0);
                }
            }
        }
        if (!currentLine.isEmpty()) {
            lines.add(currentLine.toString());
        }
        return lines;
    }
}
