#!/usr/bin/env bash
# Build local distributions into dist/:
#   the-last-signal-<version>-macos-universal.zip   (The Last Signal.app, Apple silicon + Intel)
#   the-last-signal-<version>-windows-x64.zip       (the-last-signal.exe)
#
# Run on macOS from anywhere:   scripts/dist.sh [macos|windows|all]   (default: all)
# The Windows build is cross-compiled and needs the MinGW-w64 linker:
#   brew install mingw-w64
# On a Windows machine, use dist.cmd instead.
set -euo pipefail
cd "$(dirname "$0")/.."

what="${1:-all}"
version="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
name="the-last-signal"
out="dist"
docs=(README.md START-HERE.md LICENSE config.example.json assets/fonts/OFL.txt)
mkdir -p "$out"

need_target() {
  rustup target list --installed | grep -qx "$1" || {
    echo "==> Installing Rust target $1"
    rustup target add "$1"
  }
}

# Replace one of our own earlier outputs, and nothing else.
fresh() {
  case "$1" in
    "$out/$name-"*) rm -rf -- "$1" ;;
    *) echo "refusing to remove $1" >&2; exit 1 ;;
  esac
}

build_macos() {
  [[ "$(uname -s)" == "Darwin" ]] || { echo "The macOS build must run on macOS." >&2; return 1; }
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    need_target "$t"
    echo "==> Building $t"
    cargo build --locked --release --target "$t"
  done
  local stage="$out/$name-$version-macos-universal"
  local app="$stage/The Last Signal.app"
  fresh "$stage"; fresh "$stage.zip"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
  lipo -create -output "$app/Contents/MacOS/$name" \
    "target/aarch64-apple-darwin/release/$name" \
    "target/x86_64-apple-darwin/release/$name"
  cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>The Last Signal</string>
  <key>CFBundleDisplayName</key><string>The Last Signal</string>
  <key>CFBundleIdentifier</key><string>io.github.brucehoppe.the-last-signal</string>
  <key>CFBundleExecutable</key><string>$name</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.role-playing-games</string>
  <key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
  # Ad-hoc signature: enough to run locally on Apple silicon. Not notarized, so a
  # downloaded copy needs right-click > Open the first time.
  codesign --force --sign - "$app" >/dev/null
  cp "${docs[@]}" "$stage/"
  (cd "$out" && zip -qry "$(basename "$stage").zip" "$(basename "$stage")")
  echo "==> $stage.zip"
}

build_windows() {
  local t=x86_64-pc-windows-gnu
  if ! command -v x86_64-w64-mingw32-gcc >/dev/null; then
    echo "Skipping Windows: the MinGW-w64 linker is missing. Install it with: brew install mingw-w64" >&2
    return 1
  fi
  need_target "$t"
  echo "==> Building $t"
  CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo build --locked --release --target "$t"
  local stage="$out/$name-$version-windows-x64"
  fresh "$stage"; fresh "$stage.zip"
  mkdir -p "$stage"
  cp "target/$t/release/$name.exe" "$stage/"
  cp "${docs[@]}" "$stage/"
  (cd "$out" && zip -qr "$(basename "$stage").zip" "$(basename "$stage")")
  echo "==> $stage.zip"
}

status=0
case "$what" in
  macos) build_macos ;;
  windows) build_windows ;;
  all) build_macos || status=1; build_windows || status=1 ;;
  *) echo "usage: scripts/dist.sh [macos|windows|all]" >&2; exit 2 ;;
esac
(cd "$out" && shasum -a 256 ./*.zip > SHA256SUMS-dist.txt 2>/dev/null) || true
ls -lh "$out"/*.zip 2>/dev/null || true
exit $status
