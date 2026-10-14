# Markdown 0.18.1：HTML 图片 width/height

日期：2026-10-14。状态：修复、打包与定向原生验收通过；本地候选尚未发布。

用户截图的 `<img width="128" height="128">` 被按原图大小显示。最小复现 `--plugin-cargo ../Markdown-for-Nanobug/Cargo.toml test --lib html_image_dimensions_reach_native_layout` 连续两次失败：公开图片节点的 width 为 None，预期为 Some(128)。HTML 过滤器只保留 src/alt，没有传递尺寸。

宿主原生像素测试另确认，即使通用图片节点已指定 128×128，640×320 图片仍画成 128×64；补齐像素区域后，又暴露图片说明仍被限制在同一个 128 高容器内。三处分别修复：插件读取整数像素属性；通用宿主按声明尺寸选择比例；说明保留独立布局空间。未添加插件 ID 分支或专属 API。

只指定 width 或 height 时按原图比例补齐；两者均指定时采用声明比例，空间更窄时整体缩小。没有尺寸时继续采用原图尺寸并适应预览宽度，不扩大原图。尺寸范围沿用公开布局的 0～10000；百分比、CSS 单位、负数、NaN、小数及超限属性逐项忽略。

实际制品：

- Cargo、manifest.json、plugin.toml 版本为 0.18.1，protocol 7，权限与能力要求保持不变。
- 从本机匹配宿主 `C:/Projects/RustProjects/Editor/target/debug/editor-app.exe` 的公开 CLI 构建；SDK 摘要 `a7a66395ad257bb31cd68706568ebecd6c6d62359cca86b155e149607ee0186c`。
- `dist/markdown-0.18.1.zip` SHA-256：`4ae2aa098d2d9f5fca4db4a4e55b2dd19b7d49bb246161aaa630b75de1997934`。旧 0.18.0 本地候选与已审核 0.17.2 字节保留。
- 宿主需要本次通用图片显示修复。历史 host-sdk.json 保留已发布 SDK 来源；本轮使用实际匹配的本机宿主二进制，未宣称已验证历史固定宿主的这些新显示行为。

实际验证：

- 完整插件单元：83 passed，0 failed，0 ignored；尺寸解析回归由红转绿，包含单边、零值、上限和非法属性。
- 通用 GPUI 图片几何：1 passed，实际像素区域覆盖 128×128、仅宽、仅高、放大、原始尺寸与零高，说明单独占据空间。
- 真实 Package → Manager → 原生文档：1 passed；1024×1024 图片显示为精确 128×128，并可通过真实图片点击打开 next.md，源码文件未改写。
- 既有宽图随分栏缩放并保持比例的真实 WASM 回归：1 passed。
- 独立格式检查、定向宿主 Rust 文件格式检查与两仓库 diff 检查通过；主 workspace 格式检查仍报其他任务 plugin_preview_activation.rs 的差异。
- 宿主 build 与 workspace check 通过。非 UI workspace 测试第一次因 MSVC 无法打开 msvcrt.lib（LNK1104）失败；按原命令附加 -j 1 重试后通过，ignored 项未在该命令执行，不计为通过。

宿主验证记录在本机主项目 `docs/plugins/verification/markdown-image-dimensions.md`。未安装到用户 profile、提交、推送或发布；未执行人工 OS IME 或 macOS/Linux 验收。
