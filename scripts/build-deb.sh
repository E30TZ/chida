#!/bin/sh
# Build a Debian package (chida_<version>_amd64.deb) for Chida.
#
# Usage:
#   scripts/build-deb.sh
#
# Inputs (environment):
#   VERSION   Package version, without any "v" prefix (default: 1.0.0)
#   BINARY    Path to the built chida binary (default: target/release/chida)
#   OUTDIR    Where the .deb is written (default: dist-release)
#   ARCH      Debian architecture (default: amd64)
#
# Requires: dpkg-deb (from the dpkg-dev package on Debian and Ubuntu).
set -eu

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"

VERSION="${VERSION:-1.0.0}"
BINARY="${BINARY:-target/release/chida}"
OUTDIR="${OUTDIR:-dist-release}"
ARCH="${ARCH:-amd64}"

if [ ! -f "$BINARY" ]; then
    echo "error: binary not found: $BINARY" >&2
    echo "build it first: cargo build --release" >&2
    exit 1
fi
if ! command -v dpkg-deb >/dev/null 2>&1; then
    echo "error: dpkg-deb not found; install the dpkg-dev package" >&2
    exit 1
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

mkdir -p "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/512x512/apps" \
         "$STAGE/usr/share/doc/chida" \
         "$STAGE/DEBIAN"

install -m 755 "$BINARY" "$STAGE/usr/bin/chida"

# Same desktop entry transformation as the release tarball: point Exec at
# the installed binary name and do not force a terminal window.
sed -e 's|^Exec=.*|Exec=chida|' \
    -e 's|^Terminal=.*|Terminal=false|' \
    packaging/chida.desktop > "$STAGE/usr/share/applications/chida.desktop"
chmod 644 "$STAGE/usr/share/applications/chida.desktop"

install -m 644 apps/desktop/src-tauri/icons/icon.png \
    "$STAGE/usr/share/icons/hicolor/512x512/apps/chida.png"
install -m 644 LICENSE "$STAGE/usr/share/doc/chida/copyright"

cat > "$STAGE/DEBIAN/control" <<EOF
Package: chida
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Maintainer: E30TZ
Homepage: https://github.com/E30TZ/chida
Description: Visual full-stack builder written in Rust
 Chida is a visual full-stack builder. Design pages, data tables and
 logic flows in one project, then export a render of the design or a
 complete Rust application: an Axum backend, a Leptos frontend, the SQL
 schema and an OpenAPI description.
 .
 This package installs the chida command line tool.
EOF

mkdir -p "$OUTDIR"
# Normalise permissions: dpkg-deb requires the control directory to be
# between 0755 and 0775, and data directories should ship as 0755 rather
# than whatever the local umask produced.
find "$STAGE" -type d -exec chmod 755 {} +
dpkg-deb --build --root-owner-group "$STAGE" "$OUTDIR/chida_${VERSION}_${ARCH}.deb"
echo "Built $OUTDIR/chida_${VERSION}_${ARCH}.deb"
