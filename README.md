# Drafft.ink


<img src="./logo.png" alt="Drafft.ink Logo" width="120" align="left">

**An infinite canvas whiteboard built with Rust and WebGPU.**

Try it now: [drafft.ink](https://drafft.ink/) — draw first, sign up never.

Cross-platform (Linux, Windows, macOS, browser, mobile). Real-time collaboration via CRDTs. No account required. Self-hostable with a single binary.

<br clear="left"/>

<img width="874" alt="Screenshot" src="./screenshot.png" />

---

## Features

- **Shapes and Drawing** - Rectangles, ellipses, lines, arrows, freehand paths with pressure sensitivity
- **Smart Guides** - Smart alignment snapping, equal spacing detection, angle snapping
- **Text** - Multiple font families (GelPen, GelPen Serif, Vanilla Extract), per-character styling, inline LaTeX math
- **Images** - Drag-and-drop, paste from clipboard, embedded in document
- **Collaboration** - Real-time sync via Loro CRDT. Watch your colleagues draw boxes around things that don't need boxes.
- **Open Formats** - Export to PNG or JSON. Import them back.
- **No Telemetry** - We don't know what you're drawing, and frankly, we don't want to.
- **Touch Support** - iPad and tablet friendly, gesture navigation
- **Sketch Style** - Sketchy on purpose. Precise when it matters. Hand-drawn aesthetic via roughr and fonts

---

## Installation

### Desktop

Requires a [Rust toolchain](https://rustup.rs/).

#### Linux (recommended)

```bash
git clone https://github.com/PatWie/drafft-ink.git
cd drafft-ink
./install.sh
```

This builds a release binary, installs it to `~/.cargo/bin/drafftink`, and
adds a desktop entry and icon under `~/.local/share`, so **DrafftInk appears in
your application launcher** (GNOME, KDE, Walker, Rofi, Wofi, Fuzzel, etc.) with
its icon in docks and taskbars. Search for "DrafftInk", "whiteboard" or "drawing".

To update, `git pull` and run `./install.sh` again. To remove everything:

```bash
./install.sh --uninstall
```

To keep the unmodified upstream version alongside, run
`./install.sh --original` (or `--original <git ref>`). It builds that version
into `~/.local/share/drafftink-original` and adds a separate
**DrafftInk (Original)** launcher entry. Both open the same documents; pen
strokes drawn with a brush other than Pen show as plain pen strokes in the
original.

<details>
<summary>Manual install (what the script does)</summary>

```bash
cargo install --path crates/drafftink-app --locked
install -Dm644 packaging/linux/drafftink.svg \
    ~/.local/share/icons/hicolor/scalable/apps/drafftink.svg
sed "s|^Exec=drafftink|Exec=$HOME/.cargo/bin/drafftink|" \
    packaging/linux/drafftink.desktop > ~/.local/share/applications/drafftink.desktop
update-desktop-database ~/.local/share/applications
```

The `Exec` path is made absolute because launchers often don't have
`~/.cargo/bin` on their `PATH`.
</details>

Graphics tablets (e.g. Wacom) work natively on Wayland, including pen
pressure, the eraser end, and the barrel buttons.

**Menu → Settings...** has three tabs:

- **Keyboard** — rebind any shortcut (click a key to remove it, **+** to add one).
- **Pen** — pressure curve, stabilizer, tilt/calligraphy, eraser and hover. These
  apply while a Wacom tablet is connected, or to any tablet if you force them.
- **Buttons & dials** — pen buttons, tablet keys, dials, touch ring/strip and
  pinch zoom, for any tablet. Each can run any command; dials and rings can run
  one command per direction (e.g. nudge left / right).

The pen tool's panel has **brushes** for writing (Pen, Pointed nib for
copperplate, Broad nib for italic and blackletter) and drawing (Brush pen,
Marker, Pencil, Soft brush). The grid button includes **Handwriting (3 lines)**
and **Calligraphy (4 lines)** practice guides, with adjustable x-height,
ascender and descender, row gap, slant and classic proportions.

Press **Q** (or bind "Tools and colours at pointer" to a pen or tablet button)
to bring the tools and colours to the pointer: tools to its left, colours above
it. Tap and click an item, or hold, move onto an item and release.

**Menu → Handwriting practice Intro / Calligraphy practice Intro** open practice
sheets in their own tab: every uppercase and lowercase letter, number and symbol,
written with each brush at each stroke width, with an empty row after each line
to copy it. Handwriting uses print letters on 3-line paper; calligraphy uses
cursive script on slanted 4-line paper. Letters come from the Hershey stroke
fonts (see `crates/drafftink-core/assets/hershey`).

Settings are saved in `~/.config/drafftink/` (`keybindings.json`, `tablet.json`,
`preferences.json`).

If your compositor doesn't forward tablet dials (e.g. Hyprland 0.56), DrafftInk
reads them from the device directly; allow that once with
`./install.sh --tablet-access` (installs a udev rule for tablet pads only).

#### Other platforms / run without installing

```bash
cargo run --release
```

Or use the build script:

```bash
./build.sh --native
```

### Web (Local)

```bash
./build.sh --wasm
```

### Collaboration Server

```bash
cargo build --release -p drafftink-server
./target/release/drafftink-server
```

Listens on `ws://localhost:3030/ws`. One binary, no configuration files. Self-host it. Or don't. We're not your manager.

---

## Architecture

```
crates/
  drafftink-core/     # Canvas state, shapes, CRDT sync, snapping logic
  drafftink-render/   # Vello-based GPU rendering, text layout (Parley)
  drafftink-app/      # Application logic, UI (egui), event handling
  drafftink-server/   # WebSocket collaboration server
  drafftink-widgets/  # Custom UI components
```

---

## Philosophy

Your tools should work for you. No accounts, no paywalls, no telemetry, no "upgrade to Pro."

---

## Contributing

PRs welcome. Issues welcome. The code is right here.

---

## License

**AGPLv3** - Use it, modify it, host it. Keep it open.
