#!/usr/bin/env python3
"""Intake of the desktop article-editor atlas with the upstream flow's atlas.py.

The upstream image-to-appcard-flow intake requires 8–12 scenes per atlas and its
runner (flow.py) only accepts the 406×776 phone artboard. The desktop editor
atlas has four 1280×800 scenes. This wrapper loads the unmodified upstream
atlas.py and relaxes only the scene-count check (to 4–12); cropping,
reversible transforms, hashing and immutability checks are upstream's.

    python3 lab/article-editor/intake_desktop.py \
        --flow /path/to/Octoscript-AppCard-flow
"""
import argparse, types
from pathlib import Path

HERE = Path(__file__).resolve().parent
CHECK = "not 8 <= len(scenes) <= 12"

def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument('--flow', required=True, type=Path, help='Octoscript-AppCard-flow checkout')
    args = parser.parse_args()
    upstream = args.flow / 'lab/image-to-appcard-flow/atlas.py'
    source = upstream.read_text()
    if source.count(CHECK) != 1:
        raise SystemExit(f'{upstream}: scene-count check changed upstream; review this wrapper')
    atlas = types.ModuleType('atlas')
    atlas.__file__ = str(upstream)
    exec(compile(source.replace(CHECK, "not 4 <= len(scenes) <= 12"), str(upstream), 'exec'), atlas.__dict__)
    atlas.main(['--manifest', str(HERE / 'image-to-appcard-flow-v2-desktop.json'), '--project', str(HERE),
                '--output', str(HERE / 'pipeline-output/intake-v2-desktop')])

if __name__ == '__main__':
    main()
