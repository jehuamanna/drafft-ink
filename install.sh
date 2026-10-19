#!/bin/bash
# Install DrafftInk for the current user, including a desktop entry and icon
# so it shows up in application launchers (Linux).
#
#   ./install.sh              build and install
#   ./install.sh --uninstall  remove everything installed by this script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
DESKTOP_FILE="$DATA_DIR/applications/drafftink.desktop"
ICON_FILE="$DATA_DIR/icons/hicolor/scalable/apps/drafftink.svg"
BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

refresh_caches() {
    update-desktop-database "$DATA_DIR/applications" 2>/dev/null || true
    gtk-update-icon-cache -q -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
}

if [[ "$1" == "--uninstall" ]]; then
    cargo uninstall drafftink-app 2>/dev/null || true
    rm -f "$DESKTOP_FILE" "$ICON_FILE"
    refresh_caches
    echo "DrafftInk uninstalled."
    exit 0
fi

cd "$SCRIPT_DIR"
cargo install --path crates/drafftink-app --locked --force

# Launchers often don't have ~/.cargo/bin on PATH, so point at the binary directly.
mkdir -p "$(dirname "$DESKTOP_FILE")"
sed "s|^Exec=drafftink|Exec=$BIN_DIR/drafftink|" packaging/linux/drafftink.desktop > "$DESKTOP_FILE"
install -Dm644 packaging/linux/drafftink.svg "$ICON_FILE"
refresh_caches

echo "DrafftInk installed: $BIN_DIR/drafftink"
echo "It should now appear in your application launcher."
