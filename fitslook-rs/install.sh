#!/usr/bin/env bash
set -euo pipefail

# ══════════════════════════════════════════════════════════════════════════════
# FITSLook Installer / Uninstaller
# ══════════════════════════════════════════════════════════════════════════════
#
# Usage:
#   ./install.sh                     Install to ~/.local (user-local)
#   ./install.sh /usr/local          Install to /usr/local (system-wide, needs sudo)
#   ./install.sh --uninstall         Uninstall from ~/.local
#   ./install.sh --uninstall /usr    Uninstall from /usr
# ══════════════════════════════════════════════════════════════════════════════

UNINSTALL=false
PREFIX="$HOME/.local"

for arg in "$@"; do
    case "$arg" in
        --uninstall) UNINSTALL=true ;;
        *)           PREFIX="$arg"  ;;
    esac
done

BINDIR="$PREFIX/bin"
DATADIR="$PREFIX/share"
APPDIR="$DATADIR/applications"
ICONDIR="$DATADIR/icons/hicolor/256x256/apps"
MIMEDIR="$DATADIR/mime/packages"

APP_NAME="fitslook"
DESKTOP_FILE="fitslook.desktop"
MIME_FILE="fitslook-fits.xml"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ICON_SRC="$SCRIPT_DIR/../app_icon.png"
BINARY_SRC="$SCRIPT_DIR/target/release/$APP_NAME"

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
# UNINSTALL
# ══════════════════════════════════════════════════════════════════════════════
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

# ══════════════════════════════════════════════════════════════════════════════
# INSTALL
# ══════════════════════════════════════════════════════════════════════════════
echo ""
echo -e "${BOLD}══════════════════════════════════════════${NC}"
echo -e "${BOLD}  Installing FITSLook to $PREFIX${NC}"
echo -e "${BOLD}══════════════════════════════════════════${NC}"
echo ""

# Check binary exists
if [ ! -f "$BINARY_SRC" ]; then
    err "Release binary not found at: $BINARY_SRC"
    err "Run 'cargo build --release' first, or use 'make install'."
    exit 1
fi

# ── Binary ────────────────────────────────────────────────────────────────────
install -Dm755 "$BINARY_SRC" "$BINDIR/$APP_NAME"
info "Binary  → $BINDIR/$APP_NAME"

# ── Icon ──────────────────────────────────────────────────────────────────────
if [ -f "$ICON_SRC" ]; then
    install -Dm644 "$ICON_SRC" "$ICONDIR/$APP_NAME.png"
    info "Icon    → $ICONDIR/$APP_NAME.png"
else
    info "Icon source not found ($ICON_SRC), skipping icon install."
fi

# ── Desktop Entry ─────────────────────────────────────────────────────────────
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

# ── MIME Type ─────────────────────────────────────────────────────────────────
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

# ── Update caches ─────────────────────────────────────────────────────────────
update-desktop-database "$APPDIR" 2>/dev/null || true
update-mime-database "$DATADIR/mime" 2>/dev/null || true
gtk-update-icon-cache -f -t "$DATADIR/icons/hicolor" 2>/dev/null || true

# ── Done ──────────────────────────────────────────────────────────────────────
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
