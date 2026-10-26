#!/bin/sh
# Builds dist/btrbk-nas-<version>.spk: a static x86_64 binary plus the
# interface, packed the way DSM 7's Package Center expects.
set -eu
cd "$(dirname "$0")"

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
target=x86_64-unknown-linux-musl

cargo test --quiet
cargo build --release --quiet --target "$target"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/pkg/bin" "$work/spk"

cp "target/$target/release/btrbk-nas" "$work/pkg/bin/"
cp -r ui "$work/pkg/ui"
tar -C "$work/pkg" --owner=0 --group=0 -czf "$work/spk/package.tgz" bin ui

sed "s/@VERSION@/$version/" spk/INFO.in > "$work/spk/INFO"
cp -r spk/conf spk/scripts "$work/spk/"
cp ui/images/icon_72.png "$work/spk/PACKAGE_ICON.PNG"
cp ui/images/icon_256.png "$work/spk/PACKAGE_ICON_256.PNG"

mkdir -p dist
out="dist/btrbk-nas-$version.spk"
tar -C "$work/spk" --owner=0 --group=0 -cf "$out" INFO package.tgz conf scripts PACKAGE_ICON.PNG PACKAGE_ICON_256.PNG
echo "$out"
