# 标书审核 Silver Benchmark v1

本目录用于验证标书审核系统的“问题发现能力”，包含 50 份公开招标文件、150 条人工设计的合规/合同风险注入项，以及自动评分工具。

## 目录

- `sources/`：下载的原始公开 PDF（默认不提交 Git）
- `mutated/`：在原 PDF 末页追加 3 条测试条款后的 PDF（默认不提交 Git）
- `data/source_manifest.csv|jsonl`：来源清单、下载状态、页数、哈希
- `data/annotations.csv|jsonl`：逐条真值标注
- `data/taxonomy.csv`：错误类型、严重度、匹配别名、法规依据
- `data/annotation.schema.json`：真值 JSON Schema
- `data/prediction.schema.json`：系统输出 JSON Schema
- `data/predictions.example.jsonl`：系统输出示例
- `evaluate.py`：计算 Precision、Recall、F1、Critical 检出率和 Critical 标记召回率
- `build_dataset.py`：下载、校验并生成测试 PDF 和标注

## 重要边界

这是 **Silver Benchmark**：错误是人为设计并追加到公开文件末页的，不代表原招标文件真实存在这些违法违规问题。评分器只评价注入页，原文其他页上系统发现的问题列入 `out_of_scope_predictions`，不计为误报。

因此，本数据集适合证明：

- 系统能否识别已知问题；
- 严重问题是否容易漏检；
- 同一版本迭代前后的指标变化。

它不能单独替代政府采购专业人员对原始 50 份文件进行穷尽式双人复核。要把结果作为正式上线依据，还需要对原文开展盲标、复核和分歧仲裁。

## 生成数据

在仓库根目录运行：

```powershell
python benchmark/build_dataset.py
```

首次运行会下载公开 PDF。重复运行会复用已下载文件，并校验 PDF 头、页数和 SHA-256。

## 系统输出格式

每行一个 JSON，兼容 Rust 审核结果字段：

```json
{"document_id":"BID-001","risk_id":"R_001","severity":"high","is_critical":true,"critical_reason":"地域注册限制会不合理排除外地供应商","risk_type":"地域限制","category_code":"C01_LOCAL_REGISTRATION","source_quote":"投标人须在采购人所在地注册...","reason":"以注册地限制供应商","suggestion":"删除地域限制","page_number":120,"section_path":["附加测试条款"],"legal_basis":["政府采购法实施条例第二十条"]}
```

## 评分

```powershell
python benchmark/evaluate.py `
  --gold benchmark/data/annotations.jsonl `
  --pred your_predictions.jsonl `
  --output benchmark/results/metrics.json
```

默认按 `document_id + 注入页 + 风险类型别名 + 引文相似度` 做一对一匹配。上线门槛：

- `F1 >= 0.80`
- `Critical 标记召回率 >= 0.95`（重大问题既要被发现，也要输出 `is_critical=true`）

评分结果还会给出逐文档、逐类型、逐严重度指标及漏检清单。

## 无人值守验收

双击 `run_acceptance.cmd`，程序会自动：

1. 启动 Rust 审核引擎；
2. 依次上传 `test` 集 10 份测试 PDF；
3. 定位每份文件的注入页并运行全部审核 Agent；
4. 轮询等待审核完成，失败后保留断点；
5. 汇总系统 finding、自动评分并生成 `benchmark/results/<run-id>/summary.md`。

命令行等价写法：

```powershell
powershell -ExecutionPolicy Bypass -File benchmark/run_acceptance.ps1
```

先跑 1 份探针：

```powershell
powershell -ExecutionPolicy Bypass -File benchmark/run_acceptance.ps1 `
  -Split test -Scope injected -RunId probe -Limit 1
```

完整文档模式（费用和耗时显著更高）：

```powershell
powershell -ExecutionPolicy Bypass -File benchmark/run_acceptance.ps1 `
  -Split test -Scope full
```
