# Markdown 0.17.2：独立市场交付回归修复

本记录属于 [市场工单 04](https://github.com/T-miracle/Nanobug-Plugin-Marketplace/issues/5) 的真实包验收，不改变历史工单或 0.17.1 的已审核附件。

## 固定输入与改动

- 插件 ID 为 `markdown`，版本为 `0.17.2`，ZIP 名为 `markdown-0.17.2.zip`。
- 业务源码固定在 `8480eb38e059d575cb5dafd740ed14f035d20936`；其前序修复提交为 `903c570694a64df3d8ca66d12ac5c4177592e924`、`d6de8715c137ad771d9fbfafe11d3601eff4a656`。
- 公开宿主/SDK 来源保持 `b77e6317bf7e3d3d3d0fe9490cabd362d266e0a5`；SDK SHA-256 为 `dcac05a1312eb110b7343b8b460fd9d5b6cefe3ad27753e78c519b7b85779e4c`。未添加宿主路径依赖或 SDK 源码副本。
- 完整 ZIP 为 616350 字节，SHA-256 为 `cac840637b5348c553da810e0d101f318088714caec18e30710bbc97c8327e13`。

局部重解析保留任务框的节点身份、键盘焦点和最新选中状态。派生预览在解析器构造前检查 256 KiB 输入边界，超限时返回只读预览限制并保留可编辑源码、文档版本和原生文本；不提高 WASM fuel 或宿主节点配额。限额缓存清理后，即使 SDK 语言值为空，小文档和空文档也能恢复正常预览。

## 实际验证

Windows x86_64 上直接调用公开 `--plugin-cargo Cargo.toml test --lib` 和 `--plugin-package . --output dist`；代码生成与归档不调用旧脚本。源码格式检查通过。

| 检查 | 结果 |
| --- | --- |
| 空语言/限额缓存恢复的新增回归 | 修复前在回到正常空文档时失败；修复后通过 |
| 完整插件单元测试 | 76 passed / 0 failed / 0 ignored |
| 实际 ZIP 的 Package 校验与固定输入摘要 | 通过；上述完整 ZIP 与宿主夹具逐字节一致 |
| 宿主全部 Markdown 原生集成 | 88 passed / 0 failed，实际 WASM 包已执行 |
| Standards / Spec 两轴独立审查 | 0 项未解决问题 |

原生集成来自宿主 `codex/marketplace-versions` 的 `target/marketplace-04-app-all-final21.log`，逐项核对该二进制列出的 88 个 `extensions::markdown_tests` 用例全部完成。覆盖任务框连续 Space、嵌套/大写标记与 Undo/Redo、源码高度、长文预览、工具栏、链接与双向定位、图片、中文输入/组合事件，以及双语、主题和缩放组合。宿主其他场景的全量结果由 [宿主 PR #101](https://github.com/T-miracle/Nanobug/pull/101) 及市场工单 04 的交付记录固定，不将 Markdown 子集结果当作整个 workspace 通过。

## 发布规则与限制

新版本使用独立 `v0.17.2` 标签和完整 ZIP 附件，经市场 PR 收录并固定摘要。`v0.17.1`、原附件和历史注册数据保持原值；后续构建即使来自相同源码也不得替换已审核同名 ZIP。哈希校验只能确认收到的字节匹配审核记录，不能单凭哈希证明构建来源。

本记录验证 Windows 原生自动化输入及布局，不声称人工操作已安装 GUI 的物理 OS IME 候选窗口或 macOS/Linux 验收。
