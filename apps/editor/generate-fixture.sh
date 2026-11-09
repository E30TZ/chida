#!/bin/sh
# Regenerate the editor's bundled template fixtures and thumbnails.
# The fixtures are NOT hand-written: the Chida CLI builds each project
# from the chida-core model (crates/chida-cli/src/templates.rs and
# demo.rs), and this script stores the full bundle (project, database
# and flows) minified under fixtures/tpl-<name>.json. Thumbnails are
# the CLI's own render of each template's home page, cropped to a card
# ratio, stored under assets/thumbs/<name>.jpg. The template photo
# assets are bundled under apps/editor/assets/digikala (copied from
# templates/assets/digikala) and served by Trunk via the copy-dir link
# in index.html, so the editor canvas paints the real photos.
set -e
cd "$(dirname "$0")/../.."
export PATH="$HOME/.cargo/bin:$PATH"
OUT="$(mktemp -d)"
TEMPLATES="shop digikala persian liquid-glass saas-dashboard clinic restaurant portfolio"
mkdir -p "$OUT/projects" "$OUT/renders"
for t in $TEMPLATES; do
    cargo run -q -p chida-cli -- new "tpl-$t" --template "$t" --out "$OUT/projects" >/dev/null
    mkdir -p "$OUT/renders/$t"
    cargo run -q -p chida-cli -- render "$OUT/projects/tpl-$t.chida.json" --out "$OUT/renders/$t" >/dev/null
done
python3 - "$OUT" <<'PYEOF'
import json, os, sys
from PIL import Image
out = sys.argv[1]
names = ["shop", "digikala", "persian", "liquid-glass", "saas-dashboard", "clinic", "restaurant", "portfolio"]
os.makedirs("apps/editor/assets/thumbs", exist_ok=True)
for n in names:
    data = json.load(open(f"{out}/projects/tpl-{n}.chida.json"))
    with open(f"apps/editor/fixtures/tpl-{n}.json", "w") as f:
        json.dump(data, f, ensure_ascii=False, separators=(",", ":"))
    src = f"{out}/renders/{n}/home.png"
    if not os.path.exists(src):
        src = f"{out}/renders/{n}/persian.png"
    im = Image.open(src).convert("RGB")
    w = 640
    im = im.resize((w, int(im.height * w / im.width)))
    maxh = int(w * 0.62)
    if im.height > maxh:
        im = im.crop((0, 0, w, maxh))
    im.save(f"apps/editor/assets/thumbs/{n}.jpg", quality=82)
    print("wrote fixture + thumb for", n)
PYEOF
rm -rf apps/editor/assets/digikala
cp -r templates/assets/digikala apps/editor/assets/digikala
echo "copied photo assets to apps/editor/assets/digikala"
