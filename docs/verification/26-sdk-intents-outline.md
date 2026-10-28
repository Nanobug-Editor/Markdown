# 26 — EditorIntent 接入与原生标题大纲

日期：2026-10-21～22。本地候选 `0.18.5`；实现与交付验收完成，未发布或替换用户安装。

本轮在保留 0.18.2 既有改动的基础上，为格式操作与显式导航分别接入公开 SDK `EditorIntent`。保留 30 秒、TryTerminate、文档/revision/选区验证与一次替换/Undo；清理失败保留 owner 并阻止替代分配。Prepare 独立清理两个意图，保留图片导入副作用回执。未改动 PreferenceBinding，未增加 HttpTask 或权限。

新增公开 `language.structure ^1` 提供者 `structure`，wire 仍为 7。ATX H1～H6、Setext H1/H2、跳级/重复/中文标题、行内文本、精确 UTF-8 定义与章节折叠都从不可变源码推导；代码块与原始 HTML 不生成标题。纯 worker Prepare 不发布 UI，大纲不申请编辑资源。未保存编辑刷新原生树，点击子标题定位到实际正文。1 MiB 结构源码上限与 256 KiB 预览独立，结构及回复配额超限整体失败。

大文档先保守压缩纯正文续行，再交给原有 CommonMark。完整段落中的引用/Setext 语法、ASCII 空行边界及原始 UTF-8 映射均保留；差分回归覆盖强调、链接、代码、HTML、NBSP/全角空格、引用、列表与 CRLF。

A3 已在独立副本实际试接。相同 21 个位置仍为 2 次请求、相同末尾块；双向驱动和导航抢占共 4 次请求，晚到回执增加 0 次。接入增加了清理状态，没有简化 pending+latest，故保留产品 scrolling.rs 原实现；不宣称性能提升。

固定宿主 `Editor/target/plugin-developer-tooling/debug/editor-app.exe` SHA-256：`c82565746b49011761884604da17e1bcbd30a45cb972b42a2a6ba20fda940856`；SDK：`8e2f359fedf819d8e8b6b3da1012722629e12d9731c02c38e5b01ccab8bd02e4`。使用 `--plugin-cargo` 的受管依赖，不指向宿主源码，不复制 SDK。

验证入口：

```powershell
& $pluginHost --plugin-cargo Cargo.toml test --lib
$env:NANOBUG_MARKDOWN_SDK_PACKAGE = '<实际 ZIP 绝对路径>'
& $pluginHost --plugin-cargo tests/intent-host/Cargo.toml test
& $pluginHost --plugin-package '<独立源码目录>' --output '<新归档目录>'
& $pluginHost --plugin-check '<新 ZIP>' --format json
```

插件单元 83 passed、公开 WIT 真包测试 23 passed，均为 0 ignored。接近 1 MiB 的密集短行/长段落分别消耗 87,138,814 / 69,606,296 fuel，低于不变的 100M 上限，末尾代码标题定位正确。原生大纲、生产 StructureProvider 的近上限输入、热更新与生命周期结果整理于配套主仓库专属记录 `docs/plugins/verification/official-sdk-markdown-2026-10-21.md`。原生热更新已证明 PID/文档/未保存中文/UndoRedo/私有数据保留、失败准备不换旧实例、停用移除后旧新 owner 清空。

最终 0.18.5 的两个原生大纲/热更新测试均通过，包含生产 StructureProvider 的接近 1 MiB 输入。`--plugin-dev --report` 的 admission/startup/activation/cleanup/status 均 passed、checkpoint.partial=false、正常 stop 退出 0。business=not_exercised 保留报告的设计含义，业务依据原生测试；此前 0.18.4、故意拒绝输入或 checkpoint 不完整的报告也保留，未混为同一包或改报通过。

最终候选 `dist/markdown-0.18.5.zip`，SHA-256 `383a0d5be242eac1c47be54bf219f4c0ba9ce314beb94d5ede0924a39f0aafe0`。保留旧本地 0.18.4（`76ee83cadeeac7ade066ee6c6f085e97db61788c968788166b78c933d75dbe07`），收尾用它复现独立 CR 换行错误，0.18.5 已增加 LF/CRLF/CR 全范围及折叠回归。保留旧 `dist/markdown-0.18.2.zip`，SHA-256 `61a648070f6d659ffbadd9cf8a7f8da04110ba47d007a55f6ddcd28c93a31ca2`。
