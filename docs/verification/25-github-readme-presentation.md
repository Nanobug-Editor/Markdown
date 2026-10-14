# Markdown 0.18.2：GitHub README 图片语义与样式

日期：2026-10-14。本记录保存提交前的本地实现、打包及原生验收；远程交付候选的独立复验见文末。

用户要求图片居中、成功加载时不显示 alt，并使样式与 GitHub README 保持一致。HTML p/div/标题的 align 进入公开原生布局，同段多图保持独立 URI、链接和身份；width/height 与单边比例延续 0.18.1。成功图片仅以 alt 提供无障碍名称，失败时保留替代文字和本地化原因。

[GitHub Primer](https://github.com/primer/css/tree/main/src/markdown) 的正文、六级标题、H1/H2 分隔线、段落间距、灰色引用、代码块及表格外观在访客定义，通过配套宿主的公开 ui.styles 1.0 渲染；用户 role 设置优先。未为本插件添加宿主专属分支、权限或 CSS 执行。配色声明为 63 项，保留现有 64 项上限。

Cargo、manifest.json、plugin.toml 版本均为 0.18.2，protocol 7；只新增 ui.styles 能力要求，权限集合不变。首次本地 `dist/markdown-0.18.2.zip` SHA-256：`61a648070f6d659ffbadd9cf8a7f8da04110ba47d007a55f6ddcd28c93a31ca2`。通过配套 Debug 宿主公开 --plugin-cargo / --plugin-package 构建，SDK 摘要 `2180e97920c48bcaca5c98805573b50c7018a46c3919038bc8a60d572f24eff8`。当时 host-sdk.json 仍为历史提交 b77e6317bf7e3d3d3d0fe9490cabd362d266e0a5；远程交付改用提供 ui.styles 的固定宿主提交，见当前 host-sdk.json。已交付旧包字节保留。

实际插件单元为 85 passed、0 failed、0 ignored；新增回归覆盖 align 作用域、多图身份和完整发布文档的样式/配色配额。两项通用 GPUI 测试验证无 alt 说明和实际字体/行高/代码几何；七项真实 Package → Manager → 原生场景通过，覆盖居中、图片链接、深浅 HTML、未保存输入、表格、逐图失败、宽图缩放、细微滚动、任务框及 Undo/Redo。独立最小 SDK 包另外验证 ui.styles 未声明时拒绝、声明后通过公开 View demand 激活，1 passed、0 ignored。完整格式、非 UI workspace 测试、workspace check、相关编辑器测试及两仓库 diff 检查通过；其余 ignored 项没有执行，不计通过。

完整宿主记录位于 [Nanobug 验收文档](https://github.com/T-miracle/Nanobug/blob/main/docs/plugins/verification/markdown-github-readme-presentation.md)。Base 行内代码比例为 0.875，Primer 为 0.85；原生字体塑形和表格列宽不保证浏览器逐像素相同。上述初次验收阶段未安装、提交、推送或发布，未验收人工 OS IME 与其他操作系统。

## 远程交付候选的独立复验

用户随后授权提交、推送和 PR 自动通过。插件候选从 main 的 d0e58e8b0d0d4f546754c1a12fc81531daa6e6b7 创建隔离工作树，配套宿主从 1379f58df76622bdf4d6a7c4167469f124d17a0b 隔离，只纳入本次相关变更。host-sdk.json 固定到已通过仓库检查及原生验收的宿主普通提交，供现有 PR CI 独立构建。

通过隔离宿主运行 --plugin-cargo Cargo.toml test --lib --locked 为 85 passed、0 failed、0 ignored；--plugin-package 成功。实际 SDK 摘要为 997d337a5c4e07aca0094244bdfc0e426e9d6a3f061adc932875074f11057a8a，新生成的 0.18.2 候选 ZIP SHA-256 为 f3b03114865812b943af91fa3858c5dc4c79462efcfe3e8916dd5cf143bfa769。原首次本地包字节未替换；新包在宿主测试账本使用 markdown-0.18.2-readme-pr.zip 的独立名称，67 项内容不包含源码、SDK 或构建目录。

新包的 8 项真实 Package → Manager → 原生测试全部通过，另有两项通用 GPUI 几何及一项最小 SDK 能力协商回归通过。宿主格式、非 UI workspace 测试（268 passed、256 ignored）、workspace check 及两仓库 diff 检查通过；ignored 项仅明确执行的用例计入通过。源码 PR 和固定 SDK 更新不发布或替换 Release，也不改变市场准入状态。
