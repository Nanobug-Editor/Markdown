# Markdown

[English](https://github.com/Nanobug-Editor/Markdown/blob/main/README.md) · **简体中文**

Markdown 源码编辑、原生预览、格式工具栏和同步滚动。 插件 ID 为 `markdown`，首次独立发布版本为 `0.17.1`；原有安装身份、设置和私有数据保持连续。

项目名为 **Markdown**。`0.18.2` 按 GitHub README 的逻辑处理图片对齐和替代文字，并采用 Primer 字号、间距及内容块样式。任务框焦点及 256 KiB 预览预算继续保留；超过预算的源码仍可正常编辑。已发布 ZIP 字节继续冻结，安装 ID 保持不变。

HTML 标题、段落、行内格式、换行和表格使用原生渲染；HTML 图片和链接沿用 Markdown 的受控图片加载与显式导航。脚本、事件属性、CSS 和嵌入页面会被过滤。支持范围为原生 HTML 子集，布局由原生渲染器决定。HTML 图片按所在内容块映射源码；暂不支持 HTML `id` 锚点、交互表单和 CSS 尺寸，普通 Markdown 标题锚点继续可用。

图片 `width`、`height` 接受 0～10000 的整数像素。两者同时指定时控制图片区域；只指定一边时按原始比例计算另一边。预览栏更窄时按比例缩小。段落、div 与标题上的 `align="left|center|right"` 控制原生内容对齐。图片加载成功后，`alt` 仅作为无障碍名称；加载中或失败时才显示替代文字。无效尺寸逐项忽略。

预览样式采用 [GitHub Primer 的 README 规则](https://github.com/primer/css/tree/main/src/markdown)：正文 16 px、1.5 倍行高、六级标题、H1/H2 分隔线、段落间距、灰色引用、带圆角的中性代码块，以及带边框和交替底色的表格。用户按 role 设置的字体和配色优先。原生字体塑形和 Base 行内代码指标与浏览器可能有细微差异。

本仓库是业务维护来源。[市场注册仓库](https://github.com/Nanobug-Editor/Plugin-Marketplace)负责审核，安装包保存在本仓库 Releases。每版都通过市场 PR 和人工审核，不以 Release 发布代替上架。

## 独立构建

使用 host-sdk.json 固定提交构建的 Nanobug 程序，该版本提供 `ui.styles 1.0`，并准备原生编译环境及清单所需目标。将 `$pluginHost` 设为该程序的绝对路径：

```powershell
# 通过宿主公开入口生成固定版本 ZIP，不复制 SDK 源码。
& $pluginHost --plugin-package . --output dist
```

WASM 项目另使用 `& $pluginHost --plugin-cargo Cargo.toml test --lib` 验证。声明式项目无需空 WASM。缺失工具会报错，不自动安装。发布前确认清单权限、目标平台、能力要求和 ZIP 的 SHA-256；不同字节必须提升版本重新审核。

## 维护与许可

迁移前的功能说明及历史验证见 [原始说明](https://github.com/Nanobug-Editor/Markdown/blob/v0.17.1/docs/source-readme.md)，新的构建和发布依据以本页及当前源文件为准。迁移保持既有业务行为。

Nanobug workspace 的 Cargo MIT 许可声明在 LICENSE 中明确保存；grammar、图标及原生依赖保留自身许可与随包 notice，不由根许可覆盖。将生成 ZIP 从原生插件管理器本地导入或待市场审核后安装，并按实际权限确认。
