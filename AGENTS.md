# Nanobug Markdown 协作约定

本仓库是 Markdown 插件的正式业务维护来源，保留插件 ID `markdown`。默认简体中文沟通；宿主只提供面向所有插件的公开能力。

## 开始工作

先检查 git status --short、README.md、manifest.json、nanobug-plugin.json 和 host-sdk.json，保留已有改动。市场规格、工单和交付记录统一见 [Nanobug Plugin Marketplace](https://github.com/T-miracle/Nanobug-Plugin-Marketplace/tree/main/docs)；本次迁移依据为工单 03，历史资料不自动授权其他任务。

## 实现和构建

修改 Markdown 业务前再读 [本插件规格](docs/spec.md)、[工单](docs/tickets/README.md)及对应 [历史验收](docs/verification/README.md)。原生预览、格式工具栏、任务框写回、双向滚动、布局偏好及图片命名必须保持规格与现有回归测试的行为。历史记录中的宿主内路径和脚本只供追溯，独立构建使用本仓库当前入口。

- 业务策略、资源和说明在本仓库修改；宿主仓库过渡源码冻结，不双份维护。
- 独立构建使用固定宿主版本的公开 --plugin-package / --plugin-cargo 入口。不得添加指向宿主 crates/ 的依赖或复制 SDK 源码；公开参数和权限见导出的源码注释。
- 若生成代码，必须同时生成用途、接口、状态与生命周期注释。使用 apply_patch 编辑；复用公开 API，不添加插件 ID 专属宿主分支。
- 修改交付包时提升清单、声明描述与 Cargo 包版本，保留既有安装 ID、设置和私有数据。权限和能力不能通过配置静默扩大。
- 根 README.md 与 README.zh-CN.md 同步。docs/source-readme.md 及迁入的旧验收是历史来源，不是本版本验收。

## 验证和发布

先运行受影响插件测试，通过固定宿主生成实际 ZIP，再以 Package / Manager 和原生宿主验证代表行为。源码、SDK、资源或依赖变化重跑受影响验证；完全相同的输入可复用制品，每个运行场景隔离数据和授权。

Release 固定源码 commit、包版本和完整 ZIP SHA-256，不替换同名同版本附件。首次及每版通过市场 PR 和人工审核；构建成功不等于上架。凭据不进入源码、日志或包。用户已授权的普通提交和本工单发布按现有约定执行，不强推、不改历史，不自动批准市场收录 PR。
