#!/bin/sh
# Regenerate fixtures/demo-project.json from the real Marketplace Pro
# project (the `digikala` CLI template). The fixture is NOT
# hand-written: the Chida CLI builds the project from the chida-core
# model (crates/chida-cli/src/templates.rs), and this script keeps the
# full bundle (design project, database and flows). The template photo
# assets are bundled under apps/editor/assets/digikala (copied from
# templates/assets/digikala, same files the CLI copies next to a
# generated project) and served by Trunk via the copy-dir link in
# index.html, so the editor canvas paints the real photos.
set -e
cd "$(dirname "$0")/../.."
export PATH="$HOME/.cargo/bin:$PATH"
OUT="$(mktemp -d)"
cargo run -q -p chida-cli -- new marketplace --template digikala --out "$OUT"
python3 -c "
import json, sys
data = json.load(open(sys.argv[1]))
with open('apps/editor/fixtures/demo-project.json', 'w') as f:
    json.dump(data, f, ensure_ascii=False, indent=2)
print('wrote apps/editor/fixtures/demo-project.json (full bundle: project, database, flows)')
" "$OUT/marketplace.chida.json"
rm -rf apps/editor/assets/digikala
cp -r templates/assets/digikala apps/editor/assets/digikala
echo "copied photo assets to apps/editor/assets/digikala"
