#!/bin/bash
# Builds dist/Synth.app (Apple Silicon + Intel) and dist/Synth.zip. Runs on macOS only.
set -euo pipefail
cd "$(dirname "$0")/.."

export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

app=dist/Synth.app
mkdir -p "$app/Contents/MacOS"
lipo -create -output "$app/Contents/MacOS/Synth" \
  target/aarch64-apple-darwin/release/synth \
  target/x86_64-apple-darwin/release/synth
cp macos/Info.plist "$app/Contents/Info.plist"

# Apple Silicon refuses to run unsigned code. "-" is an ad-hoc signature: no certificate needed.
codesign --force --sign - "$app"

ditto -c -k --keepParent "$app" dist/Synth.zip
