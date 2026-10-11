#!/bin/sh
# Regenerates the screenshots of the help (crates/rusteau-gui/help/img) and the offline HTML help (docs/help).
#
# The interface runs its screenshot tour (RUSTEAU_SCREENSHOTS, see crates/rusteau-gui/src/shots.rs): it loads the
# built-in examples, runs them, shows every illustrated page and saves a PNG of the window. On a machine without a
# display the tour runs under Xvfb with software OpenGL. The PNGs are reduced to 256 colours with Pillow when it is
# installed (about a quarter of the size).
#
# Usage: tools/help_screenshots.sh            (from the root of the repository)
#        RUSTEAU_SCREENSHOTS_TOPICS=1 tools/help_screenshots.sh   also renders every help topic (for proof-reading)
set -e
cargo build --release -p rusteau-gui
shots=$(mktemp -d)
if [ -n "$DISPLAY" ]; then
    RUSTEAU_SCREENSHOTS="$shots" target/release/rusteau
else
    LIBGL_ALWAYS_SOFTWARE=1 RUSTEAU_SCREENSHOTS="$shots" xvfb-run -a -s "-screen 0 1600x1000x24" target/release/rusteau
fi
copy_plain() {
    for f in "$shots"/*.png; do
        case "$(basename "$f")" in
            topic_*) ;;
            *) cp "$f" crates/rusteau-gui/help/img/ ;;
        esac
    done
}
python3 -I - "$shots" crates/rusteau-gui/help/img <<'EOF' || copy_plain
import os, sys, glob
from PIL import Image
src, dst = sys.argv[1], sys.argv[2]
for p in sorted(glob.glob(os.path.join(src, "*.png"))):
    name = os.path.basename(p)
    if name.startswith("topic_"):
        continue
    img = Image.open(p).convert("RGB").quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    img.save(os.path.join(dst, name), optimize=True)
    print(name)
EOF
if [ -n "$RUSTEAU_SCREENSHOTS_TOPICS" ]; then
    echo "renders of the help topics: $shots/topic_*.png"
else
    rm -rf "$shots"
fi
# the screenshots are compiled into the program: build again before writing the HTML help
cargo build --release -p rusteau-gui
target/release/rusteau --export-help docs/help
