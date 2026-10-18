# Markdown specification fixtures

- `commonmark-0.31.2.json`: https://spec.commonmark.org/0.31.2/spec.json — 652 examples, downloaded 2026-09-22. SHA-256: `d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20`.
- `gfm-spec.txt`: https://raw.githubusercontent.com/github/cmark-gfm/0.29.0.gfm.13/test/spec.txt — GitHub Flavored Markdown 0.29. SHA-256: `7d8e5814befec287ac116786d81ff14e0adc9b13295b4494649e995408fd871c`.
- `gfm-0.29.json`: extracted fenced examples from that GFM text, preserving example numbers and section names. The test runs its 24 extension cases (tables, strikethrough, task lists, autolinks, and disallowed raw HTML). Its inherited CommonMark 0.29 cases are superseded by the 0.31.2 suite above; we do not claim to run all 672 GFM examples.

Specification authors: John MacFarlane and the CommonMark contributors; GitHub and the GFM contributors. Specifications and these redistributed/derived fixtures are licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). JSON extraction is a format conversion, not an editorial change to the examples.

Parser tests normalize only equivalent HTML attribute ordering and void-element slash spelling. They do not assert pixel layout or browser CSS support. Native widget tests and the instrumented Rinx runner cover the rendering layer separately.
