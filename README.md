# Markdown for Nanobug

**English** · [简体中文](https://github.com/T-miracle/Markdown-for-Nanobug/blob/main/README.zh-CN.md)

Markdown source editing, native preview, format toolbar and synchronized scrolling. The stable installation ID is `markdown`; the first independent release is `0.17.1`.

The project is named **Markdown for Nanobug**. Version `0.17.2` preserves task checkbox focus across local edits and rejects oversized preview work before parsing. Source documents remain editable above the 256 KiB preview budget. The published `v0.17.1` source and ZIP bytes remain frozen; the installation ID is unchanged.

This repository is the authoritative plugin source. The [marketplace registry](https://github.com/T-miracle/Nanobug-Plugin-Marketplace) reviews submissions; fixed ZIP packages live in this repository's Releases. Publishing a Release does not list it automatically. Every initial submission and version requires a registry PR and human review.

## Build independently

Use Nanobug built from the commit pinned in host-sdk.json, with the required native toolchain and WASM target already available. Set `$pluginHost` to the absolute path of that executable:

```powershell
# The public host entry supplies its SDK and packages the project without a source copy.
& $pluginHost --plugin-package . --output dist
```

For a WASM project, run `& $pluginHost --plugin-cargo Cargo.toml test --lib` as well. Declarative packages need no empty WASM component. Missing tools are reported, never installed automatically. Before publishing, check permissions, platform claims, capability requirements and the SHA-256 of the complete ZIP. Changed bytes require a new version and review.

## Maintenance and licensing

[Original plugin notes](https://github.com/T-miracle/Markdown-for-Nanobug/blob/v0.17.1/docs/source-readme.md) preserve feature details and prior verification; current source and this README define the independent build. The migration preserves existing IDs, settings and private data.

The Nanobug workspace's Cargo MIT declaration is made explicit in LICENSE. Grammar, icons and native dependencies keep their own licenses and notices. Import a generated ZIP through the native manager, or install from the marketplace after review, and approve its actual permissions.
