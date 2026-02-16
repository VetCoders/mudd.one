#!/usr/bin/env bash
# build-dmg.sh — Build mudd.app (Release) + package as DMG
# Supports ad-hoc signing and notarization via vista-build profile
# Created by M&K (c)2026 VetCoders
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

# Config
APP_NAME="mudd"
BUNDLE_ID="io.vetcoders.mudd"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
DMG_NAME="${APP_NAME}-${VERSION}-arm64"
BUILD_DIR="${REPO_ROOT}/build"
APP_PATH="${BUILD_DIR}/${APP_NAME}.app"
DMG_PATH="${BUILD_DIR}/${DMG_NAME}.dmg"
SIGNING_IDENTITY="${SIGNING_IDENTITY:--}"
NOTARY_PROFILE="${NOTARY_PROFILE:-vista-build}"
KEYS_DIR="${HOME}/.keys"

echo "=== mudd.one DMG builder ==="
echo "Version: ${VERSION}"
echo "Signing: ${SIGNING_IDENTITY}"
echo "Notary profile: ${NOTARY_PROFILE} (not used with ad-hoc)"

# Step 1: Rust release build + Swift bindings
echo ""
echo "=== [1/5] Rust release build ==="
cargo build -p mudd-ffi --release

echo "=== [2/5] Swift bindings ==="
cargo run -p uniffi-bindgen -- generate \
    --library target/release/libmudd_ffi.dylib \
    --language swift \
    --out-dir app/mudd/Bridge/

# Step 3: Xcode project + build
echo ""
echo "=== [3/5] Xcode build (Release) ==="
cd app && xcodegen generate 2>/dev/null && cd ..

# Clean build dir
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"

DERIVED_DATA="${BUILD_DIR}/DerivedData"

set -o pipefail
xcodebuild \
    -project app/mudd.xcodeproj \
    -scheme mudd \
    -configuration Release \
    -derivedDataPath "${DERIVED_DATA}" \
    CODE_SIGN_IDENTITY="${SIGNING_IDENTITY}" \
    CODE_SIGN_STYLE=Manual \
    PRODUCT_BUNDLE_IDENTIFIER="${BUNDLE_ID}" \
    build 2>&1 | tail -5

# Find the built .app
BUILT_APP=$(find "${DERIVED_DATA}" -name "${APP_NAME}.app" -type d | head -1)
if [ -z "$BUILT_APP" ]; then
    echo "ERROR: ${APP_NAME}.app not found in DerivedData"
    exit 1
fi

cp -R "${BUILT_APP}" "${APP_PATH}"
echo "App: ${APP_PATH}"

# Step 4: Code sign
echo ""
echo "=== [4/5] Code signing ==="
if [ "${SIGNING_IDENTITY}" = "-" ]; then
    echo "Ad-hoc signing..."
    codesign --force --deep --sign - "${APP_PATH}"
    echo "Signed (ad-hoc)"
else
    echo "Signing with: ${SIGNING_IDENTITY}"
    codesign --force --deep --options runtime --sign "${SIGNING_IDENTITY}" "${APP_PATH}"
    codesign --verify --deep --strict "${APP_PATH}"
    echo "Signed + verified"
fi

# Step 5: Create DMG
echo ""
echo "=== [5/5] Creating DMG ==="
rm -f "${DMG_PATH}"

# Create temp dir for DMG contents
DMG_STAGING="${BUILD_DIR}/dmg-staging"
rm -rf "${DMG_STAGING}"
mkdir -p "${DMG_STAGING}"
cp -R "${APP_PATH}" "${DMG_STAGING}/"
ln -s /Applications "${DMG_STAGING}/Applications"

hdiutil create \
    -volname "${APP_NAME}" \
    -srcfolder "${DMG_STAGING}" \
    -ov \
    -format UDZO \
    "${DMG_PATH}" 2>/dev/null

rm -rf "${DMG_STAGING}"

# Sign DMG too
if [ "${SIGNING_IDENTITY}" = "-" ]; then
    codesign --force --sign - "${DMG_PATH}"
else
    codesign --force --sign "${SIGNING_IDENTITY}" "${DMG_PATH}"
fi

echo ""
echo "=== Done ==="
echo "DMG: ${DMG_PATH}"
echo "Size: $(du -h "${DMG_PATH}" | cut -f1)"
echo ""

# Notarization instructions (for later)
if [ "${SIGNING_IDENTITY}" != "-" ]; then
    echo "To notarize (when ready):"
    echo "  xcrun notarytool submit '${DMG_PATH}' --keychain-profile '${NOTARY_PROFILE}' --wait"
    echo "  xcrun stapler staple '${DMG_PATH}'"
    echo ""
    echo "To store notary credentials first:"
    echo "  xcrun notarytool store-credentials '${NOTARY_PROFILE}' \\"
    echo "    --key '${KEYS_DIR}/AuthKey_ZT6AXN3759.p8' \\"
    echo "    --key-id ZT6AXN3759 \\"
    echo "    --issuer <YOUR_ISSUER_ID>"
fi
