# Math notation check (upyesp.org)

This checks the 11 tables of
[Markdown math notation](https://www.upyesp.org/posts/makrdown-vscode-math-notation/)
in the article editor: 195 formulas, 145 distinct.

- **Source:** `source.md` puts each table's inline forms in a Markdown table,
  followed by the display forms. A first table shows column widths set by
  delimiter dashes (3:6:2).
- **Pipeline check:** every formula reaches the math renderer intact through
  `markdown_render::render`, including those with `|` in table cells. Each was
  parsed with `makepad-latex-math`, with a 2-second hang guard.
- **Preview:** `preview-*.png` are the editor's 预览 in instrument mode (hidden
  window), scrolled top to bottom.

## Fixed in makepad (OctoSense-org/makepad `rinx/math-parser-fixes`)

- **Hang:** any `]` or stray `}` made the parser loop forever. For example,
  `[\ldots] (\ldots)` or even `$[a]$` froze Rinx, which typesets on the UI thread.
- **Missing commands** that were printed literally: `\cup \cap \otimes \dag
  \degree \rq \colon`, `\cr` rows, and `\Alpha … \Chi`. Also added:
  `\sqcup \sqcap \uplus \vee \wedge \oplus \ominus \oslash \odot \ddag \prime
  \therefore \because \lbrack \rbrack`.
- **Delimiters:** `\left< … \right>` are angle brackets.
- **Matrices:** whitespace before `\end{…}` no longer leaves a stray `\end`.
- **Typesetting:** `\mathrm` is upright, and a hyphen draws as a minus sign.
- **Table cells:**
  - Math, images and diagrams in cells drew at about one pixel wide, because
    the width is NaN while laying out.
  - They now fit the cell.
- **Column widths:** delimiter dashes set relative column widths.

## Still not rendered

- `P(A \ca pB)` and `P(A\capB)`: typos on the page itself; KaTeX rejects them too.
- Other `\mathbf`/`\mathsf`/… variants are laid out in the default style.
