import fs from "node:fs/promises";
import path from "node:path";
import { SpreadsheetFile, Workbook } from "@oai/artifact-tool";

const root = path.resolve("benchmark");
const outputDir = path.resolve("outputs", "019f91e9-23df-7233-b7ee-e04f24dbc4b5");
const outputPath = path.join(outputDir, "标书审核基准排查表.xlsx");

async function readJsonl(filePath) {
  const text = await fs.readFile(filePath, "utf8");
  return text.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line));
}

const sources = await readJsonl(path.join(root, "data", "source_manifest.jsonl"));
const annotations = await readJsonl(path.join(root, "data", "annotations.jsonl"));
const taxonomyCsv = await fs.readFile(path.join(root, "data", "taxonomy.csv"), "utf8");
const taxonomyWorkbook = await Workbook.fromCSV(taxonomyCsv, { sheetName: "错误分类" });
const taxonomyValues = taxonomyWorkbook.worksheets.getItem("错误分类").getUsedRange().values;

const workbook = Workbook.create();
const overview = workbook.worksheets.add("概览");
const sourceSheet = workbook.worksheets.add("来源清单");
const annotationSheet = workbook.worksheets.add("真值标注");
const taxonomySheet = workbook.worksheets.add("错误分类");
taxonomySheet.getRangeByIndexes(
  0, 0, taxonomyValues.length, taxonomyValues[0]?.length ?? 1
).values = taxonomyValues;

const navy = "#17365D";
const blue = "#1F4E78";
const paleBlue = "#D9EAF7";
const paleYellow = "#FFF2CC";
const paleGreen = "#E2F0D9";
const paleRed = "#FCE4D6";
const border = "#D9E2F3";

overview.showGridLines = false;
overview.getRange("A1:F1").merge();
overview.getRange("A1").values = [["标书审核 Silver Benchmark v1"]];
overview.getRange("A1:F1").format = {
  fill: navy,
  font: { color: "#FFFFFF", bold: true, size: 18 },
  horizontalAlignment: "center",
  verticalAlignment: "center",
};
overview.getRange("A1:F1").format.rowHeight = 36;

overview.getRange("A2:F2").merge();
overview.getRange("A2").values = [[
  "50份公开招标文件｜150条合成注入真值｜评分范围仅限追加测试页"
]];
overview.getRange("A2:F2").format = {
  fill: paleBlue,
  font: { color: navy, italic: true },
  horizontalAlignment: "center",
};

overview.getRange("A4:A10").values = [
  ["数据项"], ["文档数"], ["标注总数"], ["Critical 数"], ["Train 文档"], ["Dev 文档"], ["Test 文档"]
];
overview.getRange("B4:B10").values = [
  ["数量"], [null], [null], [null], [null], [null], [null]
];
overview.getRange("B5").formulas = [["=COUNTA('来源清单'!A2:A51)"]];
overview.getRange("B6").formulas = [["=COUNTA('真值标注'!D2:D151)"]];
overview.getRange("B7").formulas = [["=COUNTIF('真值标注'!H2:H151,\"critical\")"]];
overview.getRange("B8").formulas = [["=COUNTIF('来源清单'!B2:B51,\"train\")"]];
overview.getRange("B9").formulas = [["=COUNTIF('来源清单'!B2:B51,\"dev\")"]];
overview.getRange("B10").formulas = [["=COUNTIF('来源清单'!B2:B51,\"test\")"]];
overview.getRange("A4:B10").format.borders = { preset: "all", style: "thin", color: border };
overview.getRange("A4:B4").format = {
  fill: blue, font: { color: "#FFFFFF", bold: true }, horizontalAlignment: "center"
};
overview.getRange("A5:A10").format.fill = paleBlue;
overview.getRange("B5:B10").format.horizontalAlignment = "center";

overview.getRange("D4:E4").values = [["上线门槛", "要求"]];
overview.getRange("D5:E7").values = [
  ["F1", 0.80],
  ["Critical Recall", 0.95],
  ["同时满足", "F1与Critical Recall"],
];
overview.getRange("D9:E9").values = [["实测结果", "填写"]];
overview.getRange("D10:E12").values = [
  ["实测 F1", null],
  ["实测 Critical Recall", null],
  ["门禁判断", null],
];
overview.getRange("E12").formulas = [["=IF(AND(E10>=E5,E11>=E6),\"通过\",\"不通过\")"]];
overview.getRange("D4:E7").format.borders = { preset: "all", style: "thin", color: border };
overview.getRange("D9:E12").format.borders = { preset: "all", style: "thin", color: border };
overview.getRange("D4:E4").format = { fill: blue, font: { color: "#FFFFFF", bold: true } };
overview.getRange("D9:E9").format = { fill: blue, font: { color: "#FFFFFF", bold: true } };
overview.getRange("D10:D12").format.fill = paleBlue;
overview.getRange("E10:E11").format.fill = paleYellow;
overview.getRange("E12").format = { fill: paleGreen, font: { bold: true }, horizontalAlignment: "center" };
overview.getRange("E5:E6").setNumberFormat("0%");
overview.getRange("E10:E11").setNumberFormat("0.00%");

overview.getRange("A14:F14").merge();
overview.getRange("A14").values = [["使用说明"]];
overview.getRange("A14:F14").format = { fill: navy, font: { color: "#FFFFFF", bold: true } };
overview.getRange("A15:F18").merge(true);
overview.getRange("A15:F18").values = [
  ["1. 用 mutated/ 目录中的 PDF 运行审核，系统结果导出为 JSONL。"],
  ["2. 运行 evaluate.py，只评价每份 PDF 最后一页的人工注入问题。"],
  ["3. 原始 PDF 其他页的发现不计误报；正式上线仍需对原文做双人穷尽标注。"],
  ["4. 本表中的法规依据用于基准设计，发布或执法使用前应由政府采购专业人员复核。"],
];
overview.getRange("A15:F18").format = {
  fill: "#F7F9FC", wrapText: true, font: { color: "#404040" },
  borders: { preset: "all", style: "thin", color: border },
};
overview.getRange("A:A").format.columnWidth = 24;
overview.getRange("B:B").format.columnWidth = 14;
overview.getRange("C:C").format.columnWidth = 4;
overview.getRange("D:D").format.columnWidth = 24;
overview.getRange("E:E").format.columnWidth = 22;
overview.getRange("F:F").format.columnWidth = 4;

const sourceHeaders = [
  "document_id", "split", "title", "purchaser_or_source", "publication_year", "domain",
  "source_url", "source_host", "download_status", "original_pages", "injection_page",
  "source_sha256", "mutated_sha256", "mutated_file"
];
const sourceRows = sources.map((row) => sourceHeaders.map((key) => row[key] ?? ""));
sourceSheet.getRangeByIndexes(0, 0, sourceRows.length + 1, sourceHeaders.length).values = [
  sourceHeaders, ...sourceRows
];
sourceSheet.freezePanes.freezeRows(1);
sourceSheet.freezePanes.freezeColumns(2);
sourceSheet.getRangeByIndexes(0, 0, 1, sourceHeaders.length).format = {
  fill: navy, font: { color: "#FFFFFF", bold: true }, wrapText: true
};
sourceSheet.getRangeByIndexes(0, 0, sourceRows.length + 1, sourceHeaders.length).format.borders = {
  preset: "all", style: "thin", color: border
};
sourceSheet.getRange("A:N").format.wrapText = true;
for (const col of ["A", "B", "E", "I", "J", "K"]) sourceSheet.getRange(`${col}:${col}`).format.columnWidth = 14;
sourceSheet.getRange("C:D").format.columnWidth = 28;
sourceSheet.getRange("F:F").format.columnWidth = 18;
sourceSheet.getRange("G:G").format.columnWidth = 52;
sourceSheet.getRange("H:H").format.columnWidth = 28;
sourceSheet.getRange("L:M").format.columnWidth = 34;
sourceSheet.getRange("N:N").format.columnWidth = 26;
sourceSheet.tables.add(`A1:N${sourceRows.length + 1}`, true, "SourceManifest");

const annotationHeaders = [
  "benchmark_version", "split", "document_id", "finding_id", "annotation_origin",
  "category_code", "risk_type", "severity", "is_critical", "page_number", "section_path",
  "source_quote", "reason", "suggestion", "legal_basis", "match_aliases"
];
const annotationRows = annotations.map((row) => [
  row.benchmark_version, row.split, row.document_id, row.finding_id, row.annotation_origin,
  row.category_code, row.risk_type, row.severity, row.is_critical, row.page_number,
  row.section_path.join(" / "), row.source_quote, row.reason, row.suggestion,
  row.legal_basis.join(" | "), row.match_aliases.join(" | ")
]);
annotationSheet.getRangeByIndexes(0, 0, annotationRows.length + 1, annotationHeaders.length).values = [
  annotationHeaders, ...annotationRows
];
annotationSheet.freezePanes.freezeRows(1);
annotationSheet.freezePanes.freezeColumns(4);
annotationSheet.getRangeByIndexes(0, 0, 1, annotationHeaders.length).format = {
  fill: navy, font: { color: "#FFFFFF", bold: true }, wrapText: true
};
annotationSheet.getRangeByIndexes(0, 0, annotationRows.length + 1, annotationHeaders.length).format.borders = {
  preset: "all", style: "thin", color: border
};
annotationSheet.getRange("A:P").format.wrapText = true;
annotationSheet.getRange("A:K").format.columnWidth = 17;
annotationSheet.getRange("L:P").format.columnWidth = 38;
annotationSheet.tables.add(`A1:P${annotationRows.length + 1}`, true, "GoldAnnotations");

taxonomySheet.freezePanes.freezeRows(1);
const taxonomyUsed = taxonomySheet.getUsedRange();
taxonomyUsed.format.borders = { preset: "all", style: "thin", color: border };
taxonomyUsed.format.wrapText = true;
taxonomySheet.getRange("A:I").format.columnWidth = 24;
taxonomySheet.getRange("F:I").format.columnWidth = 40;
taxonomySheet.getRange("A1:I1").format = {
  fill: navy, font: { color: "#FFFFFF", bold: true }, wrapText: true
};

await fs.mkdir(outputDir, { recursive: true });
const output = await SpreadsheetFile.exportXlsx(workbook);
await output.save(outputPath);

const preview = await workbook.render({
  sheetName: "概览",
  range: "A1:F18",
  scale: 1.5,
  format: "png",
});
await fs.writeFile(path.join(outputDir, "标书审核基准排查表_preview.png"), new Uint8Array(await preview.arrayBuffer()));

const inspection = await workbook.inspect({
  kind: "sheet,region,formula",
  sheetId: "概览",
  range: "A1:F18",
  maxChars: 5000,
});
await fs.writeFile(path.join(outputDir, "workbook_inspection.json"), inspection.ndjson ?? String(inspection), "utf8");

console.log(JSON.stringify({
  outputPath,
  sheets: ["概览", "来源清单", "真值标注", "错误分类"],
  sources: sources.length,
  annotations: annotations.length,
}, null, 2));
