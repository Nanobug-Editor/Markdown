# Markdown

[English](https://github.com/Nanobug-Editor/Markdown/blob/main/README.md) · **简体中文**

Markdown 源码编辑、原生预览、格式工具栏和同步滚动。 插件 ID 为 `markdown`，首次独立发布版本为 `0.17.1`；原有安装身份、设置和私有数据保持连续。

项目名为 **Markdown**。`0.17.2` 保留局部编辑后任务框的键盘焦点，并在解析前限制过量预览工作；超过 256 KiB 预览预算的源码仍可正常编辑。已发布的 `v0.17.1` 源码和 ZIP 字节继续冻结，安装 ID 保持不变。

本仓库是业务维护来源。[市场注册仓库](https://github.com/Nanobug-Editor/Plugin-Marketplace)负责审核，安装包保存在本仓库 Releases。每版都通过市场 PR 和人工审核，不以 Release 发布代替上架。

## 独立构建

使用 host-sdk.json 固定版本构建的 Nanobug 程序，并准备原生编译环境及清单所需目标。将 `$pluginHost` 设为该程序的绝对路径：

```powershell
# 通过宿主公开入口生成固定版本 ZIP，不复制 SDK 源码。
& $pluginHost --plugin-package . --output dist
```

WASM 项目另使用 `& $pluginHost --plugin-cargo Cargo.toml test --lib` 验证。声明式项目无需空 WASM。缺失工具会报错，不自动安装。发布前确认清单权限、目标平台、能力要求和 ZIP 的 SHA-256；不同字节必须提升版本重新审核。

## 维护与许可

迁移前的功能说明及历史验证见 [原始说明](https://github.com/Nanobug-Editor/Markdown/blob/v0.17.1/docs/source-readme.md)，新的构建和发布依据以本页及当前源文件为准。迁移保持既有业务行为。

Nanobug workspace 的 Cargo MIT 许可声明在 LICENSE 中明确保存；grammar、图标及原生依赖保留自身许可与随包 notice，不由根许可覆盖。将生成 ZIP 从原生插件管理器本地导入或待市场审核后安装，并按实际权限确认。
