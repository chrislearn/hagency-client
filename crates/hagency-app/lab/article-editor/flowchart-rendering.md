# Flowchart previews

Rinx now renders the supplied Editor.md `flow` fence in both its native full
preview and its Makepad + Blitz preview. The four Chinese nodes, yes/no branches,
and the return arrow to 登陆操作 are preserved. `flowchart` is an alias.
The diagram remains editable Markdown; changing 用户登陆 to 提交订单 regenerates
both previews, and the exact edited source survives saving and restarting.

The shared Rust parser lays out static SVG, which resvg rasterizes to a 2× PNG.
The native reader displays the generated image through ArticleDiagram; Blitz
receives the same pixels through its existing image grants. No browser or remote
JavaScript library is needed in Rinx. The reference WebView comparison renders
the same generated SVG with WebKit, so it compares rendering, not an independent
flowchart layout implementation. The identical-HTML mode shares the PNG.

[WebView vs Makepad + Blitz](render-comparison/index.html) defaults to the diagram
section. All twenty browser pairs load, all ten section captures are complete,
and the full document retains the existing 8192 CSS-pixel Blitz height limit.
The native full preview can scroll through the complete document to End.

Supported shapes are start, end, operation, condition, input, output,
inputoutput and subroutine. The layout handles chains, branches, joins, cycles,
and self-loops. Labels are escaped, Chinese text is preserved, and `\n` breaks
labels into lines. Layout is automatic; a left hint can move a return edge.
This is a bounded subset of the [flowchart.js syntax](https://github.com/adrai/flowchart.js).
Flow states, node links and parallel nodes remain unsupported in the legacy
flowchart syntax. Mermaid and Editor.md sequence rendering were subsequently
added; see [native diagram support](native-diagrams.md).
Errors display an explanation and original code rather than a partial diagram.

Parser/image tests cover the exact Chinese fixture, four arrows, branch labels,
changed image bytes after editing, joins, cycles, XML escaping, malformed node
references, duplicate IDs and input limits. Integration tests check that both
host renderers use the diagram while leaving source untouched.

The Makepad instrumentation test uses a hidden real Metal window and an isolated
profile. It checks Chinese node and branch-label OCR, scrolling to the end node,
changed pixels in both previews after editing, exact source after restart, and
invalid syntax fallback. The full content regression also checks images, tables,
code, emoji and math.

```sh
python3 tools/wechat-ux/live/native_article_math.py --content-checks \
  --binary /path/to/Rinx.app/Contents/MacOS/rinx \
  --profile /path/to/isolated-fixture
```

## Installed build validation

All 16 Makepad instrumentation checks passed on the signed installed executable;
all 35 visible-window samples were zero. Native component tests: 11 passed.
Rinx library tests: 157 passed, 1 ignored. No i18n keys are missing.
Original five drafts and their unapplied source data were preserved byte-for-byte.

[Actual Rinx screenshots and test receipt](render-comparison/evidence/rinx-headless-flow/index.html).
Signed executable SHA-256: `735e3964db99377a6bc3ac0d4d9c9062a80ae040aa9f08bc8580f668929b9818`.
