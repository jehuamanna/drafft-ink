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
to bring the controls to the pointer: tools to its left; fill, stroke width and
stroke colours above it; and the properties panel to its right. Press again (or
Esc, or click the canvas) to hide them. Picking a tool hides them; colours,
widths and properties stay open so you can set several.

**Tabs and files.** Each tab is a document. Ctrl+N or Ctrl+T (or "+ New") opens
an empty one, Ctrl+O opens a file in a new tab (or switches to it if it's already
open), Ctrl+S saves to the tab's file (asking where only the first time),
Ctrl+Shift+S saves elsewhere, Ctrl+W closes the tab, and Ctrl+Tab /
Ctrl+Shift+Tab switch tabs. A dot (●) marks unsaved changes; you're asked
before closing such a tab or quitting. Right-click a tab to save, rename or
close it (double-click renames, middle-click closes). Menu → Open Recent lists
recent files, and the tabs open when you quit come back on the next start.

**Menu → Handwriting practice ›** and **Calligraphy practice ›** open practice
sheets in their own tab: every uppercase and lowercase letter, number and symbol
in a lettering style, on matching guide lines. Each line is followed by the
same letters dotted, to trace over, and then an empty row to write it yourself.

- Handwriting: Print, Slanted print, Clean print, Clean italic print, Casual
  hand, Typewriter print, School cursive, Serif print (trace)
- Calligraphy: Copperplate script, Formal script, Chancery, Slanted roman
  (italic hand), Italic, Decorative, Blackletter, Fraktur, Rotunda, Formal
  script and Italic (the last five to trace)

Most styles show every brush at every stroke width; "trace" styles show
outlined letters, to trace and copy with any brush. Letters come from the Hershey and EMS
single-line fonts (credits and licences in `crates/drafftink-core/assets/practice`).

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

## Differences from upstream

This fork is based on [PatWie/drafft-ink](https://github.com/PatWie/drafft-ink)
at `8ce40ab` (2026-07-06), the current upstream `main`. It adds to upstream
rather than rewriting it: nothing upstream has been merged back, and upstream
has no newer commits to merge in.

| Measure | Amount |
|---|---|
| Commits ahead of upstream | 10 (0 behind) |
| Files changed | 50: 30 added, 19 modified, 1 removed |
| Rust code | +8,039 / −1,525 lines, against about 30,500 upstream: roughly a quarter more code, about 5% of upstream's lines changed or removed |
| Data | +1,826 lines of stroke-font data for the practice sheets |
| Packaging and docs | `install.sh`, desktop entry, icon, udev rule, this README |

(As of `40f67ef`. To recount: `git diff --shortstat 8ce40ab...main`.)

**New in this fork**

- Graphics tablets on Wayland: pen pressure, tilt, eraser end, barrel
  buttons, tablet keys, touch ring/strip, dials (read from the device when the
  compositor doesn't forward them), pinch zoom; pressure curve, stabilizer and
  hover settings. Applies to Wacom tablets, or any tablet if forced.
- Smoother freehand strokes (no faceting after release).
- Brushes: Pen, Pointed nib, Broad nib, Brush pen, Marker, Pencil, Soft brush.
- Handwriting and calligraphy guide grids, and 19 practice sheets.
- Configurable keyboard shortcuts, pen buttons, tablet keys and dials
  (Settings window).
- Toolbars at the pointer (tools, colours, fill, widths, properties).
- More stroke widths and a width slider; colour shortcuts.
- Tabs as documents: per-tab files, Ctrl+S to the same file, unsaved-change
  markers and prompts, recent files, session restore, New Document.
- Linux desktop install (`install.sh`) and `--original` to install upstream
  alongside.

**Changed from upstream behaviour**

- The grid button opens a menu of styles instead of cycling on each click
  (the shortcut still cycles).
- Ctrl+S saves to the tab's file instead of always asking; opened files open
  in their own tab, fitted to view, instead of replacing the current drawing.
- Closing the last tab leaves an empty one; closing or quitting with unsaved
  changes asks first.
- The pen panel's "Normal / Calligraphy" toggle is now "Motion: Direct /
  Smoothed" (it was a smoothing setting); calligraphy is the Broad nib brush.
- Ctrl+/− no longer scales the interface; they zoom the canvas only.
- Overlapping see-through strokes of the same colour no longer darken where
  they overlap (strokes are drawn in batches for speed).
- Fixed: opened files not appearing until the pointer moved.

**Compatibility.** Documents stay compatible both ways. Pen strokes gain a
`brush` and `nib_angle`; upstream ignores them and draws those strokes as
plain pen strokes, and saving from upstream drops them. Settings this fork
adds live in `~/.config/drafftink/`, which upstream doesn't read.

**Keeping up with upstream.** The fork's code lives in its own files, so
upstream's files change as little as possible and upstream changes merge with
few conflicts. In upstream's files there are mostly short calls into them:

| Upstream file | Fork code in |
|---|---|
| `drafftink-app/src/app.rs` | `app/` (`actions`, `commands`, `documents`, `extra`, `file_io`, `pen_input`) |
| `drafftink-app/src/ui.rs` | `ui/` (`menu`, `tab_strip`, `settings`, `quick_bars`, `queued`, `prompts`, `guides`, `stroke`) |
| `drafftink-render/src/vello_impl.rs` | `vello_impl/` (`brushes`, `overlays`) |
| `drafftink-render/src/renderer.rs` | `writing_guides.rs` |
| `drafftink-core/src/shapes/freehand.rs` | `shapes/brush.rs` |

To bring in upstream changes:

```bash
git remote add upstream https://github.com/PatWie/drafft-ink.git  # once
git config rerere.enabled true    # once: reuse conflict resolutions
git fetch upstream
git rebase -Xignore-space-change upstream/main
cargo fmt --all
```

(Use whatever name your remote for PatWie/drafft-ink has.)

`-Xignore-space-change` matters: the properties panel was re-indented when it
became shared with the toolbars at the pointer, and without it any upstream
change to that panel conflicts across all ~500 lines. Conflicts that remain are
usually both sides adding a line in the same place (keep both). Upstream's
keyboard shortcuts dialog and `shortcuts.rs` were removed in favour of the
Settings window; if upstream changes them, keep them removed.

---

## Philosophy

Your tools should work for you. No accounts, no paywalls, no telemetry, no "upgrade to Pro."

---

## Contributing

PRs welcome. Issues welcome. The code is right here.

---

## License

**AGPLv3** - Use it, modify it, host it. Keep it open.
