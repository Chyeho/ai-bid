# 讲师验收测试命令

> ⚠️ 只给讲师看。用于验收学员的大作业。

---

## 一、基础验收（必须通过）

```powershell
# 1. 编译检查
cd 学员的项目目录
cargo check
# 期望：0 error

# 2. 代码质量
cargo clippy -- -D warnings
# 期望：0 warning

# 3. 运行审核
cargo run -- review 测试标书.txt
# 期望：
#   - 终端输出 6 条条款的审核过程
#   - 每条条款有思考过程或工具调用日志
#   - 最终输出审核报告
#   - 生成 report.json
```

## 二、功能验收（抽查）

```powershell
# 4. 检查 report.json 是否存在
Test-Path report.json
# 期望：True

# 5. 检查 JSON 格式
# 用 PowerShell 解析，不报错即格式正确
Get-Content report.json | ConvertFrom-Json
# 期望：无报错

# 6. 检查高风险发现是否有法规引用
# 手动查看 report.json，找 severity=high 的条目
# 期望：每条 high 风险的 reason 字段有具体法规引用（如"第X条"）
```

## 三、进阶验收（选做）

```powershell
# 7. 换一个测试标书再跑
# 自己写一份不同于标准测试标书的 txt，验证 Agent 不是硬编码的
cargo run -- review 自备测试标书.txt

# 8. 让学员口头解释
# 随机选一段代码，让学员解释：
#   - 这段代码做了什么
#   - Agent Loop 在哪里
#   - 工具是怎么注册和调用的
```

## 四、常见问题判断

| 现象 | 判断 |
|---|---|
| `cargo check` 失败 | 直接打回，0 分 |
| 程序 panic 退出 | 扣 10 分 |
| report.json 为空或格式错误 | 扣 10 分 |
| 所有条款判 none | 说明 Agent 太怂，扣 10 分 |
| 所有条款判 high | 说明 Agent 乱判，扣 10 分 |
| 高风险无法规引用 | 每条扣 5 分 |
| 终端只输出结果不输出过程 | 扣 5 分（不符合"可观测"要求） |
