#!/usr/bin/env python3
"""Compare a running Rinx window grab with a v2 atlas scene's reference.

Uses the image-to-appcard-flow's compare_screens.py (side-by-side, diff regions),
which renders scenes itself; this compares grabs of the real Rinx window instead,
for the 1280×800 desktop scenes and the 406×776 mobile scenes. Evidence about pixels, not an approval: look at the side-by-side.

    python3 lab/article-editor/compare_v2.py --flow /path/to/Octoscript-AppCard-flow \
        --grab window.png --scene split-editor --out evidence/ [--caption 64]
"""
import argparse, importlib.util, json
from pathlib import Path
from PIL import Image
import numpy as np

HERE = Path(__file__).resolve().parent

def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--flow', required=True, type=Path)
    ap.add_argument('--grab', required=True, type=Path, help='PNG from the remote bridge /g')
    ap.add_argument('--scene', required=True, help='desktop scene id, e.g. split-editor')
    ap.add_argument('--out', required=True, type=Path)
    ap.add_argument('--caption', type=int, default=64, help='grab pixels of window caption above the content')
    ap.add_argument('--set', choices=['desktop', 'mobile'], default='desktop', help='which v2 atlas the scene is in')
    args = ap.parse_args()
    spec = importlib.util.spec_from_file_location('compare_screens', args.flow / 'lab/image-to-appcard-flow/compare_screens.py')
    cs = importlib.util.module_from_spec(spec); spec.loader.exec_module(cs)
    cs.SCALE = 2  # regions in points of the 2x reference
    ref_path = HERE / f'pipeline-output/intake-v2-{args.set}/scenes/article-v2-{args.scene}/reference.png'
    page = Image.open(ref_path).convert('RGB')
    grab = Image.open(args.grab).convert('RGB')
    native = grab.crop((0, args.caption, grab.width, grab.height)).resize(page.size, Image.LANCZOS)
    args.out.mkdir(parents=True, exist_ok=True)
    side = args.out / f'{args.scene}-compare.png'
    cs.side_by_side(native, page, side, args.scene)
    a = np.asarray(native).astype(int); b = np.asarray(page).astype(int)
    bands = [round(float(np.abs(a[y:y + 160] - b[y:y + 160]).mean()), 1) for y in range(0, page.height, 160)]
    report = {'scene': args.scene, 'reference': str(ref_path), 'grab': str(args.grab), 'side_by_side': str(side),
              'band_mean_diff': bands, 'diff_regions': cs.diff_regions(native, page, limit=12)}
    (args.out / f'{args.scene}-compare.json').write_text(json.dumps(report, indent=1))
    print(json.dumps({k: report[k] for k in ('scene', 'band_mean_diff', 'side_by_side')}))
    for r in report['diff_regions'][:8]:
        print(r)

if __name__ == '__main__':
    main()
