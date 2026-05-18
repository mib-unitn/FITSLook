#!/usr/bin/env bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════════════
# FITSLook Installer / Uninstaller
# ══════════════════════════════════════════════════════════════════════════════
#
# Usage:
#   ./install.sh                     Install (user-local on Linux, /Applications on macOS)
#   ./install.sh /custom/prefix      Install to a custom prefix (Linux only)
#   ./install.sh --uninstall         Uninstall
#   ./install.sh --uninstall /prefix Uninstall from custom prefix
# ══════════════════════════════════════════════════════════════════════════════

UNINSTALL=false
PREFIX=""

for arg in "$@"; do
    case "$arg" in
        --uninstall) UNINSTALL=true ;;
        *)           PREFIX="$arg"  ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_NAME="fitslook"
BINARY_SRC="$SCRIPT_DIR/target/release/$APP_NAME"
ICON_SRC="$SCRIPT_DIR/../app_icon.png"
OS="$(uname -s)"

# ── Colors ────────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${CYAN}▸${NC} $*"; }
ok()    { echo -e "${GREEN}✅${NC} $*"; }
err()   { echo -e "${RED}✖${NC} $*" >&2; }

# ══════════════════════════════════════════════════════════════════════════════
# macOS
# ══════════════════════════════════════════════════════════════════════════════
if [ "$OS" = "Darwin" ]; then
    APP_BUNDLE="${PREFIX:-/Applications}/AstroFITS Explorer.app"
    CONTENTS="$APP_BUNDLE/Contents"
    MACOS_DIR="$CONTENTS/MacOS"
    RESOURCES_DIR="$CONTENTS/Resources"

    if $UNINSTALL; then
        echo ""
        echo -e "${BOLD}Uninstalling AstroFITS Explorer from ${APP_BUNDLE}${NC}"
        echo "────────────────────────────────────────"
        if [ -d "$APP_BUNDLE" ]; then
            rm -rf "$APP_BUNDLE"
            info "Removed $APP_BUNDLE"
        fi
        echo ""
        ok "AstroFITS Explorer uninstalled."
        exit 0
    fi

    # ── Install ──
    echo ""
    echo -e "${BOLD}══════════════════════════════════════════${NC}"
    echo -e "${BOLD}  Installing AstroFITS Explorer (.app)${NC}"
    echo -e "${BOLD}══════════════════════════════════════════${NC}"
    echo ""

    if [ ! -f "$BINARY_SRC" ]; then
        err "Release binary not found at: $BINARY_SRC"
        err "Run 'cargo build --release' first, or use 'make install'."
        exit 1
    fi

    # Create .app bundle structure
    mkdir -p "$MACOS_DIR"
    mkdir -p "$RESOURCES_DIR"

    # Copy binary
    cp "$BINARY_SRC" "$MACOS_DIR/$APP_NAME"
    chmod 755 "$MACOS_DIR/$APP_NAME"
    info "Binary  → $MACOS_DIR/$APP_NAME"

    # Create Info.plist
    cat > "$CONTENTS/Info.plist" << 'PLIST_EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>AstroFITS Explorer</string>
    <key>CFBundleDisplayName</key>
    <string>AstroFITS Explorer</string>
    <key>CFBundleIdentifier</key>
    <string>com.fitslook.astrofits</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleExecutable</key>
    <string>fitslook</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>FITS File</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.fits</string>
            </array>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>fits</string>
                <string>fit</string>
                <string>fts</string>
            </array>
        </dict>
    </array>
    <key>UTImportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key>
            <string>public.fits</string>
            <key>UTTypeDescription</key>
            <string>FITS Astronomical Data</string>
            <key>UTTypeConformsTo</key>
            <array>
                <string>public.data</string>
            </array>
            <key>UTTypeTagSpecification</key>
            <dict>
                <key>public.filename-extension</key>
                <array>
                    <string>fits</string>
                    <string>fit</string>
                    <string>fts</string>
                </array>
            </dict>
        </dict>
    </array>
</dict>
</plist>
PLIST_EOF
    info "Plist   → $CONTENTS/Info.plist"

    # Convert icon to .icns if possible
    if [ -f "$ICON_SRC" ]; then
        if command -v sips &>/dev/null && command -v iconutil &>/dev/null; then
            ICONSET_DIR=$(mktemp -d)/AppIcon.iconset
            mkdir -p "$ICONSET_DIR"
            for SIZE in 16 32 64 128 256 512; do
                sips -z $SIZE $SIZE "$ICON_SRC" --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}.png" &>/dev/null
                DOUBLE=$((SIZE * 2))
                sips -z $DOUBLE $DOUBLE "$ICON_SRC" --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}@2x.png" &>/dev/null
            done
            iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/AppIcon.icns" 2>/dev/null || true
            rm -rf "$(dirname "$ICONSET_DIR")"
            info "Icon    → $RESOURCES_DIR/AppIcon.icns"
        else
            cp "$ICON_SRC" "$RESOURCES_DIR/AppIcon.png"
            info "Icon    → $RESOURCES_DIR/AppIcon.png (PNG, no icns conversion available)"
        fi
    else
        info "Icon source not found ($ICON_SRC), skipping icon."
    fi

    echo ""
    ok "AstroFITS Explorer installed to: $APP_BUNDLE"
    echo ""
    echo "  You can now:"
    echo "    • Open from Finder or Launchpad"
    echo "    • Right-click .fits files → Open With → AstroFITS Explorer"
    echo "    • Run from terminal: open '$APP_BUNDLE'"
    echo ""

    exit 0
fi

# ══════════════════════════════════════════════════════════════════════════════
# Linux
# ══════════════════════════════════════════════════════════════════════════════
PREFIX="${PREFIX:-$HOME/.local}"

BINDIR="$PREFIX/bin"
DATADIR="$PREFIX/share"
APPDIR="$DATADIR/applications"
ICONDIR="$DATADIR/icons/hicolor/256x256/apps"
MIMEDIR="$DATADIR/mime/packages"

DESKTOP_FILE="fitslook.desktop"
MIME_FILE="fitslook-fits.xml"

if $UNINSTALL; then
    echo ""
    echo -e "${BOLD}Uninstalling FITSLook from $PREFIX${NC}"
    echo "────────────────────────────────────────"

    [ -f "$BINDIR/$APP_NAME" ]        && rm -f "$BINDIR/$APP_NAME"        && info "Removed binary"
    [ -f "$APPDIR/$DESKTOP_FILE" ]    && rm -f "$APPDIR/$DESKTOP_FILE"    && info "Removed desktop entry"
    [ -f "$ICONDIR/$APP_NAME.png" ]   && rm -f "$ICONDIR/$APP_NAME.png"   && info "Removed icon"
    [ -f "$MIMEDIR/$MIME_FILE" ]      && rm -f "$MIMEDIR/$MIME_FILE"      && info "Removed MIME type"

    update-desktop-database "$APPDIR" 2>/dev/null || true
    update-mime-database "$DATADIR/mime" 2>/dev/null || true

    echo ""
    ok "FITSLook uninstalled."
    exit 0
fi

# ── Install ──
echo ""
echo -e "${BOLD}══════════════════════════════════════════${NC}"
echo -e "${BOLD}  Installing FITSLook to $PREFIX${NC}"
echo -e "${BOLD}══════════════════════════════════════════${NC}"
echo ""

if [ ! -f "$BINARY_SRC" ]; then
    err "Release binary not found at: $BINARY_SRC"
    err "Run 'cargo build --release' first, or use 'make install'."
    exit 1
fi

# Binary
install -Dm755 "$BINARY_SRC" "$BINDIR/$APP_NAME"
info "Binary  → $BINDIR/$APP_NAME"

# Icon
if [ -f "$ICON_SRC" ]; then
    install -Dm644 "$ICON_SRC" "$ICONDIR/$APP_NAME.png"
    info "Icon    → $ICONDIR/$APP_NAME.png"
else
    info "Icon source not found ($ICON_SRC), skipping icon install."
fi

# Desktop Entry
mkdir -p "$APPDIR"
cat > "$APPDIR/$DESKTOP_FILE" << DESKTOP_EOF
[Desktop Entry]
Name=AstroFITS Explorer
GenericName=FITS File Viewer
Comment=View astronomical FITS images, spectra, and tables
Exec=$BINDIR/$APP_NAME %u
Icon=$APP_NAME
Type=Application
Categories=Science;Astronomy;DataVisualization;
MimeType=image/fits;image/x-fits;application/fits;
Terminal=false
StartupNotify=true
Keywords=FITS;astronomy;science;spectroscopy;
DESKTOP_EOF
info "Desktop → $APPDIR/$DESKTOP_FILE"

# MIME Type
mkdir -p "$MIMEDIR"
cat > "$MIMEDIR/$MIME_FILE" << MIME_EOF
<?xml version="1.0" encoding="UTF-8"?>
<mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
  <mime-type type="image/fits">
    <comment>FITS astronomical data</comment>
    <glob pattern="*.fits"/>
    <glob pattern="*.fit"/>
    <glob pattern="*.fts"/>
  </mime-type>
</mime-info>
MIME_EOF
info "MIME    → $MIMEDIR/$MIME_FILE"

# Update caches
update-desktop-database "$APPDIR" 2>/dev/null || true
update-mime-database "$DATADIR/mime" 2>/dev/null || true
gtk-update-icon-cache -f -t "$DATADIR/icons/hicolor" 2>/dev/null || true

echo ""
ok "FITSLook installed successfully!"
echo ""
echo "  You can now:"
echo "    • Run from terminal:  $APP_NAME"
echo "    • Right-click .fits files → Open With → AstroFITS Explorer"
echo "    • Search 'AstroFITS' in your application launcher"
echo ""

# Check if BINDIR is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$BINDIR"; then
    echo -e "  ${RED}⚠${NC}  $BINDIR is not in your PATH."
    echo "     Add this to your ~/.bashrc or ~/.zshrc:"
    echo ""
    echo "       export PATH=\"$BINDIR:\$PATH\""
    echo ""
fi
