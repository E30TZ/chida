#!/bin/sh
# Assemble an AppImage for Chida (chida-<version>-linux-x86_64.AppImage).
#
# The script always assembles a complete AppDir. Turning the AppDir into
# the final .AppImage needs appimagetool:
#   * if appimagetool is on PATH (or $APPIMAGETOOL points at it), it is used;
#   * else, with FETCH_APPIMAGETOOL=1, the official AppImageKit continuous
#     build is downloaded and used (this is what CI does);
#   * otherwise the AppDir is left in place and no .AppImage is produced.
#     That final squashfs step then happens in CI only.
#
# Usage:
#   scripts/build-appimage.sh
#   FETCH_APPIMAGETOOL=1 scripts/build-appimage.sh
#
# Inputs (environment):
#   VERSION             Version, without any "v" prefix (default: 1.0.0)
#   BINARY              Built chida binary (default: target/release/chida)
#   OUTDIR              Output directory (default: dist-release)
#   APPDIR              AppDir location (default: $OUTDIR/AppDir)
#   APPIMAGETOOL        Path to an appimagetool binary (optional)
#   FETCH_APPIMAGETOOL  Set to 1 to download appimagetool when missing
#   APPIMAGE_EXTRACT_AND_RUN  Passed through to appimagetool; set to 1 in
#                       containers without FUSE (CI does this).
set -eu

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

VERSION="${VERSION:-1.0.0}"
BINARY="${BINARY:-target/release/chida}"
OUTDIR="${OUTDIR:-dist-release}"
APPDIR="${APPDIR:-$OUTDIR/AppDir}"
APPIMAGE_OUT="$OUTDIR/chida-$VERSION-linux-x86_64.AppImage"
TOOL_URL="https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage"

if [ ! -f "$BINARY" ]; then
    echo "error: binary not found: $BINARY" >&2
    echo "build it first: cargo build --release" >&2
    exit 1
fi

# --- Assemble the AppDir -------------------------------------------------
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" \
         "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/512x512/apps"

install -m 755 "$BINARY" "$APPDIR/usr/bin/chida"

# Desktop entry at the AppDir root (AppImage requirement) and in the
# standard applications directory, with the same transformation the
# release tarball uses.
sed -e 's|^Exec=.*|Exec=chida|' \
    -e 's|^Terminal=.*|Terminal=false|' \
    packaging/chida.desktop > "$APPDIR/chida.desktop"
cp "$APPDIR/chida.desktop" "$APPDIR/usr/share/applications/chida.desktop"

install -m 644 apps/desktop/src-tauri/icons/icon.png "$APPDIR/chida.png"
cp "$APPDIR/chida.png" "$APPDIR/.DirIcon"
install -m 644 apps/desktop/src-tauri/icons/icon.png \
    "$APPDIR/usr/share/icons/hicolor/512x512/apps/chida.png"

ln -sf usr/bin/chida "$APPDIR/AppRun"
echo "Assembled AppDir at $APPDIR"

# --- Locate appimagetool --------------------------------------------------
TOOL=""
if [ -n "${APPIMAGETOOL:-}" ] && [ -x "${APPIMAGETOOL:-}" ]; then
    TOOL="$APPIMAGETOOL"
elif command -v appimagetool >/dev/null 2>&1; then
    TOOL="$(command -v appimagetool)"
elif [ "${FETCH_APPIMAGETOOL:-0}" = "1" ]; then
    CACHE_DIR="${TMPDIR:-/tmp}/chida-appimage-tools"
    mkdir -p "$CACHE_DIR"
    if [ ! -x "$CACHE_DIR/appimagetool" ]; then
        echo "Downloading appimagetool from $TOOL_URL"
        curl -fL -o "$CACHE_DIR/appimagetool" "$TOOL_URL"
        chmod +x "$CACHE_DIR/appimagetool"
    fi
    TOOL="$CACHE_DIR/appimagetool"
fi

if [ -z "$TOOL" ]; then
    echo "note: appimagetool not found, so no .AppImage was produced."
    echo "The AppDir above is complete; CI builds the final .AppImage"
    echo "from it (re-run with FETCH_APPIMAGETOOL=1 to do it here)."
    exit 0
fi

# --- Build the AppImage ----------------------------------------------------
mkdir -p "$OUTDIR"
rm -f "$APPIMAGE_OUT"
ARCH=x86_64 "$TOOL" "$APPDIR" "$APPIMAGE_OUT"
chmod +x "$APPIMAGE_OUT"
echo "Built $APPIMAGE_OUT"
