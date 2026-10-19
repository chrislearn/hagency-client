"""Check native application UI for controls that bypass the shared style system.

Run from any directory: python tools/audit_desktop_styles.py
Document text, avatar initials and symbolic glyphs may use a deliberate size;
mark those lines with `style-size: content`, `avatar`, or `glyph`.
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
UI_ROOTS = (ROOT / "crates/hagency-desktop/src", ROOT / "crates/hagency-desktop/apps/article-editor/native")


def production(source):
    # Production widget recipes precede the test fixtures in these modules.
    return source.split("#[cfg(test)]", 1)[0]


def audit():
    errors = []
    count = 0
    for root in UI_ROOTS:
        for path in sorted(root.rglob("*.rs")):
            count += 1
            source = production(path.read_text(encoding="utf-8"))
            relative = path.relative_to(ROOT).as_posix()
            for line_no, line in enumerate(source.splitlines(), 1):
                code = line.split("//", 1)[0]
                if re.search(r"font_size\s*:\s*\(?\d", code) and not re.search(r"style-size: (avatar|glyph|content)", line):
                    errors.append(f"{relative}:{line_no}: use a semantic font-size role")
                if path.name not in ("theme.rs", "dropdown.rs") and re.search(r"(?<![\w.:])(?:mod\.widgets\.)?(?:Button|ButtonFlat|ButtonFlatter|TextInput|DropDown|CheckBox|RadioButton|RadioButtonFlat|Label|Html)\s*\{", code):
                    errors.append(f"{relative}:{line_no}: use a shared Rinx control")
    for message in errors:
        print(message)
    print(f"Native style audit: {count} source files, {len(errors)} violations")
    return bool(errors)


if __name__ == "__main__":
    sys.exit(audit())
