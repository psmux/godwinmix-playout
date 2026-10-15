#!/usr/bin/env bash
# Make the release archive for one platform.
#
#   dev/package.sh <platform> <dir with the built binaries> <out dir>
#
# Writes playout-<version>-<platform>.zip on Windows and .tar.gz elsewhere,
# holding one `playout/` folder with the manifest at its top, which is what
# `gmx plugin add` and the gallery install. The version is the manifest's.
set -euo pipefail
platform="$1"; bins="$(cd "$2" && pwd)"
mkdir -p "$3"; out="$(cd "$3" && pwd)"
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/gmx-plugin.toml" | head -1)"
stage="$(mktemp -d)/playout"
mkdir -p "$stage/bin"
cp -r "$root/gmx-plugin.toml" "$root/README.md" "$root/LICENSE" "$root/schemas" "$root/skills" "$root/ui" "$stage/"
case "$platform" in
    windows-*)
        cp "$bins/playout-engine.exe" "$stage/bin/"
        name="playout-$version-$platform.zip"
        (cd "$stage/.." && 7z a -tzip "$out/$name" playout > /dev/null)
        ;;
    *)
        cp "$bins/playout" "$bins/playout-engine" "$stage/bin/"
        chmod 755 "$stage/bin/playout" "$stage/bin/playout-engine"
        name="playout-$version-$platform.tar.gz"
        (cd "$stage/.." && tar -czf "$out/$name" playout)
        ;;
esac
echo "$out/$name"
