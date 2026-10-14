# Markdown 0.18.0：内嵌 HTML

日期：2026-10-14。状态：实现、81 项插件单元测试、打包与 4 项定向原生验收通过；本地候选尚未发布。

用户要求允许渲染 HTML 元素，覆盖[原规格](../spec.md)的首版排除条款。`src/preview/embedded_html.rs` 使用 HTML5 tokenizer，输出受控的原生富文本、图片和精确叶节点链接；支持标题、段落、行内格式、换行、表格、图片与链接，不创建 WebView。脚本、CSS、事件属性、嵌入页面和交互表单被过滤。未知普通容器保留可显示的文字；代码中的标签仍为字面文字。HTML 图片使用包含块的源码范围，不新增可变文档状态或权限。

Cargo、插件清单和声明版本同步为 0.18.0，协议 7 与既有公开能力保持不变。新增依赖 `html5ever = 0.27.0` 使用宿主已经采用的版本，锁定依赖并携带新依赖许可证/notice；不复制宿主 SDK 源码。双语根 README 同步说明原生子集、HTML `id` 锚点和 CSS 尺寸限制。

实际执行：

- 在宿主目录执行 `target/debug/editor-app.exe --plugin-cargo ../Markdown-for-Nanobug/Cargo.toml test --lib`：81 passed，0 failed，0 ignored。既有未处理 Result 的测试警告仍在，未扩大修改范围。
- `cargo fmt --manifest-path Cargo.toml --check`、`git diff --check`：通过。
- 在宿主目录执行 `target/debug/editor-app.exe --plugin-package ../Markdown-for-Nanobug --output ../Markdown-for-Nanobug/dist`：通过；插件包为 `dist/markdown-0.18.0.zip`，最终 SHA-256 为 `274877f66055ea95751714f9f758ef5e51bc5bfef87adec634e215e48e6db33b`。
- 当前本机公开宿主提供 SDK 摘要 `b02e41e9b1a7f92e8e745885df2388baab68cb28f487a009d3505b363b6d730f`；本次实际验证未使用 `host-sdk.json` 历史固定二进制，不将其历史摘要冒充本轮输入。
- 最终 ZIP 经主项目实际 Package/Manager 的 4 项定向原生验收通过，0 failed、0 ignored：两项 HTML 场景覆盖真实布局、图片像素、深浅主题、未保存输入和 HTML 链接；混合 Markdown 预览与普通相对链接另验证任务框、代码块、Undo/Redo、关闭清理和文档导航。
- 主项目非 UI workspace 测试、workspace 编译检查和最终 UI 测试编译均通过。并发任务曾导致 UI 测试缺少模块/导入，随后恢复。定向原生验证使用同一 deps 目录中已包含本次场景的二进制副本。完整 90 项组未运行结束，不计为通过；最新主 workspace 格式检查受并发文件影响，本次文件的定向格式检查通过。详细命令、日志和实际限制记录在本机主项目 `docs/plugins/verification/markdown-embedded-html.md`。

主项目保留 0.17.2 已审核包作为市场快照默认输入，本地候选单独进入固定账本并明确标记未提交/未发布。未改变用户安装、市场数据、已发布 ZIP、设置或私有数据；未执行 Git 提交、推送或发布。Windows 原生自动化是本轮行为证据，未宣称人工 OS IME 候选窗口或跨平台验收。
