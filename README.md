# Markdown

**English** · [简体中文](https://github.com/Nanobug-Editor/Markdown/blob/main/README.zh-CN.md)

Markdown source editing, native preview, format toolbar and synchronized scrolling. The stable installation ID is `markdown`; the first independent release is `0.17.1`.

The project is named **Markdown**. Version `0.18.2` follows GitHub README image alignment and alternative-text behavior, with Primer typography and block styling. Task focus and the 256 KiB preview budget are retained; larger source files remain editable. Published ZIP bytes remain frozen and the installation ID is unchanged.

HTML headings, paragraphs, inline formatting, line breaks and tables render natively. HTML images and links use the same controlled image loading and explicit navigation as Markdown. Scripts, event attributes, CSS and embedded pages are filtered out; this is a native HTML subset rather than a browser layout engine. HTML images share their containing block's source mapping. HTML `id` anchors, interactive forms and CSS sizing are not supported; ordinary Markdown heading anchors continue to work.

Image `width` and `height` accept integer pixels from 0 to 10000. Both set the pixel area; a single attribute preserves the original aspect ratio. A narrower preview scales it down proportionally. `align="left|center|right"` on paragraphs, divs and headings controls native content alignment. Loaded images keep `alt` as their accessible name; alternative text is visible only during loading or failure. Invalid sizes fall back per attribute.

Preview styling follows [GitHub Primer's README rules](https://github.com/primer/css/tree/main/src/markdown): 16 px body text, 1.5 line height, six heading levels, H1/H2 rules, paragraph spacing, muted quotes, neutral rounded code blocks, and bordered striped tables. User role typography/colors retain priority. Native font shaping and Base's inline-code metrics can differ slightly from a browser.

This repository is the authoritative plugin source. The [marketplace registry](https://github.com/Nanobug-Editor/Plugin-Marketplace) reviews submissions; fixed ZIP packages live in this repository's Releases. Publishing a Release does not list it automatically. Every initial submission and version requires a registry PR and human review.

## Build independently

Use Nanobug built from the commit pinned in `host-sdk.json`, which exposes `ui.styles 1.0`, with the required native toolchain and WASM target already available. Set `$pluginHost` to that executable's absolute path:

```powershell
# The public host entry supplies its SDK and packages the project without a source copy.
& $pluginHost --plugin-package . --output dist
```

For a WASM project, run `& $pluginHost --plugin-cargo Cargo.toml test --lib` as well. Declarative packages need no empty WASM component. Missing tools are reported, never installed automatically. Before publishing, check permissions, platform claims, capability requirements and the SHA-256 of the complete ZIP. Changed bytes require a new version and review.

## Maintenance and licensing

[Original plugin notes](https://github.com/Nanobug-Editor/Markdown/blob/v0.17.1/docs/source-readme.md) preserve feature details and prior verification; current source and this README define the independent build. The migration preserves existing IDs, settings and private data.

The Nanobug workspace's Cargo MIT declaration is made explicit in LICENSE. Grammar, icons and native dependencies keep their own licenses and notices. Import a generated ZIP through the native manager, or install from the marketplace after review, and approve its actual permissions.
