# Blind Benchmark v2

本目录是标书审核系统的冻结盲测集，不得作为训练集、提示词示例或规则调优样本。

## 数据构成

- 10 份与 Silver Benchmark v1 来源零重叠的公开采购文件。
- 每份追加 3 条新设计问题，共 30 条真值。
- 15 个风险类别各出现 2 次。
- 其中 10 条为 Critical，10 条为普通 High，10 条为 Medium。
- PDF 只显示“补充条款一/二/三”，不显示风险名称，避免答案泄漏。

## 冻结规则

`data/freeze_manifest.json` 保存全部原始 PDF、测试 PDF、真值和来源清单的 SHA-256。
`run_benchmark.py` 在运行前自动复核这些哈希；任一文件改变都会中止盲测。

校验命令：

```powershell
python benchmark/build_blind_v2.py --verify
```

## 首次正式结果

- 运行编号：`blind-v2-final-20260727`
- 10/10 文档完成，TP=21、FP=16、FN=9
- Precision=56.76%
- Recall=70.00%
- F1=62.69%
- Critical 检出率=70.00%
- Critical 标记召回率=30.00%
- 结论：FAIL，不满足正式上线门槛

首次运行时 BLIND-008 因 PDF 页码与解析页码偏移未能定位注入块。原始失败结果已保存在
`results/blind-v2-final-20260727/first_pass/`。随后只修复了中性页眉定位逻辑，未修改冻结数据、
审核模型、提示词或风险规则，并仅补跑 BLIND-008；其余 9 份均复用首次结果。

