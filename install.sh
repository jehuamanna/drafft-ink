#!/bin/bash
# Install DrafftInk for the current user, including a desktop entry and icon
# so it shows up in application launchers (Linux).
#
#   ./install.sh              build and install
#   ./install.sh --uninstall  remove everything installed by this script
#   ./install.sh --tablet-access
#                             allow reading tablet pad dials (asks for sudo)
#   ./install.sh --original [REF]
#                             also install the upstream version (default
#                             origin/main) as a separate "DrafftInk (Original)"

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

RULES_FILE=/etc/udev/rules.d/70-drafftink-tablet.rules
ORIGINAL_ROOT="$DATA_DIR/drafftink-original"
ORIGINAL_DESKTOP_FILE="$DATA_DIR/applications/drafftink-original.desktop"

if [[ "$1" == "--original" ]]; then
    # Build the given git ref in a temporary worktree, so the original sits
    # next to this version without touching the checkout.
    REF="${2:-origin/main}"
    cd "$SCRIPT_DIR"
    if [[ "$REF" == origin/* ]]; then
        git fetch origin "${REF#origin/}"
    fi
    WORKTREE="$(mktemp -d)"
    trap 'git -C "$SCRIPT_DIR" worktree remove --force "$WORKTREE" 2>/dev/null || true' EXIT
    git worktree add --detach "$WORKTREE" "$REF"
    CARGO_TARGET_DIR="$SCRIPT_DIR/target/original" cargo install \
        --path "$WORKTREE/crates/drafftink-app" --locked --force --root "$ORIGINAL_ROOT"

    mkdir -p "$(dirname "$ORIGINAL_DESKTOP_FILE")"
    sed -e "s|^Exec=drafftink|Exec=$ORIGINAL_ROOT/bin/drafftink|" \
        -e "s|^Name=DrafftInk|Name=DrafftInk (Original)|" \
        -e "s|^Comment=.*|Comment=Unmodified upstream DrafftInk ($REF, $(git rev-parse --short "$REF"))|" \
        -e "/^StartupWMClass=/d" \
        packaging/linux/drafftink.desktop > "$ORIGINAL_DESKTOP_FILE"
    install -Dm644 packaging/linux/drafftink.svg "$ICON_FILE"
    refresh_caches
    echo "Original DrafftInk ($REF) installed: $ORIGINAL_ROOT/bin/drafftink"
    echo "It appears in your launcher as \"DrafftInk (Original)\"."
    exit 0
fi

if [[ "$1" == "--tablet-access" ]]; then
    # Some compositors don't forward tablet pad dials; DrafftInk then reads
    # them from the device, which needs this udev rule.
    sudo install -Dm644 "$SCRIPT_DIR/packaging/linux/70-drafftink-tablet.rules" "$RULES_FILE"
    sudo udevadm control --reload
    sudo udevadm trigger --subsystem-match=input --action=change
    echo "Tablet pad access granted. Restart DrafftInk to use the dials."
    exit 0
fi

if [[ "$1" == "--uninstall" ]]; then
    cargo uninstall drafftink-app 2>/dev/null || true
    rm -f "$DESKTOP_FILE" "$ICON_FILE" "$ORIGINAL_DESKTOP_FILE"
    rm -rf "$ORIGINAL_ROOT"
    if [[ -f "$RULES_FILE" ]]; then
        sudo rm -f "$RULES_FILE" && sudo udevadm control --reload
    fi
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
