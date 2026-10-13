#!/bin/sh
# Packages the macOS build as rusteau-macos-universal.zip: the app Rusteau-1D.app and the command line program
# rusteau-cli (both universal: Apple Silicon and Intel), the HTML help and the licenses of the bundled fonts.
# Run by the build workflow on macOS after
#   cargo build --release --target aarch64-apple-darwin -p rusteau-gui -p rusteau-cli
#   cargo build --release --target x86_64-apple-darwin -p rusteau-gui -p rusteau-cli
set -e
version=$(sed -n 's/^version = "\(.*\)"/\1/p' crates/rusteau-gui/Cargo.toml | head -n 1)
rm -rf dist
app=dist/Rusteau-1D.app
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
lipo -create -output "$app/Contents/MacOS/rusteau" \
    target/aarch64-apple-darwin/release/rusteau target/x86_64-apple-darwin/release/rusteau
lipo -create -output dist/rusteau-cli \
    target/aarch64-apple-darwin/release/rusteau-cli target/x86_64-apple-darwin/release/rusteau-cli
sed "s/@VERSION@/$version/g" packaging/macos/Info.plist > "$app/Contents/Info.plist"

# the icon, from the 512 px PNG
iconset="$(mktemp -d)/icon.iconset"
mkdir -p "$iconset"
for s in 16 32 128 256 512; do
    sips -z "$s" "$s" crates/rusteau-gui/assets/icon-512.png --out "$iconset/icon_${s}x${s}.png" >/dev/null
done
for s in 16 32 128 256; do
    d=$((s * 2))
    sips -z "$d" "$d" crates/rusteau-gui/assets/icon-512.png --out "$iconset/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/icon.icns"

# an ad-hoc signature (Apple Silicon runs only signed code; the app is not notarized)
codesign --force --deep --sign - "$app"
codesign --force --sign - dist/rusteau-cli

cp -R docs/help dist/help
mkdir dist/fonts
cp crates/rusteau-gui/fonts/OFL-*.txt crates/rusteau-gui/fonts/README.md dist/fonts/
ditto -c -k dist rusteau-macos-universal.zip
lipo -info "$app/Contents/MacOS/rusteau" dist/rusteau-cli
