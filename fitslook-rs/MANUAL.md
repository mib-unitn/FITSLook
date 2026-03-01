# FITSLook-RS User Manual 🔭

A professional, high-performance FITS file viewer for astronomy — rewritten in Rust with the `egui` GUI framework.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Building from Source](#building-from-source)
3. [Running](#running)
4. [User Interface Overview](#user-interface-overview)
5. [Features](#features)
6. [Linux Desktop Integration](#linux-desktop-integration)
7. [Troubleshooting](#troubleshooting)

---

## Prerequisites

### Rust Toolchain

Install from [rustup.rs](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Minimum version: Rust 1.75+ (edition 2021).

### System Dependencies (Linux)

The GUI uses OpenGL via the system, so on Fedora/RHEL:

```bash
sudo dnf install gcc pkg-config libxkbcommon-devel wayland-devel libXcursor-devel \
    libXrandr-devel libXi-devel mesa-libGL-devel mesa-libEGL-devel
```

On Ubuntu/Debian:

```bash
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
    libxcursor-dev libxrandr-dev libxi-dev libgl-dev libegl-dev
```

---

## Building from Source

```bash
cd fitslook-rs

# Debug build (fast compile, slower runtime)
cargo build

# Release build (optimized, stripped — recommended)
cargo build --release
```

The binary will be at:
- Debug: `target/debug/fitslook`
- Release: `target/release/fitslook`

---

## Installing

### Quick Install (recommended)

```bash
cd fitslook-rs
make install
```

This builds the release binary and installs everything to `~/.local/`:

| What | Where |
|------|-------|
| Binary | `~/.local/bin/fitslook` |
| Icon | `~/.local/share/icons/hicolor/256x256/apps/fitslook.png` |
| Desktop entry | `~/.local/share/applications/fitslook.desktop` |
| MIME type | `~/.local/share/mime/packages/fitslook-fits.xml` |

After install, `fitslook` will be available in your terminal (if `~/.local/bin` is in your PATH), in your app launcher, and in the "Open With" menu for `.fits` files.

### Install script directly

```bash
cd fitslook-rs
cargo build --release
./install.sh                       # Install to ~/.local (user-local)
sudo ./install.sh /usr/local       # Install system-wide
```

### Via cargo install

```bash
cd fitslook-rs
cargo install --path .
```

This installs just the binary to `~/.cargo/bin/fitslook`. Desktop integration will be set up on first launch.

### Uninstall

```bash
cd fitslook-rs
make uninstall                     # Remove from ~/.local
# OR
./install.sh --uninstall           # Same thing
sudo ./install.sh --uninstall /usr/local   # If installed system-wide
```

---

## Running

### Open a FITS file directly

```bash
./target/release/fitslook /path/to/your/file.fits
```

### Open with a `file://` URI

```bash
./target/release/fitslook "file:///home/user/My%20Galaxy.fits"
```

Percent-encoded characters (e.g. `%20` for spaces) are handled automatically.

### Open without a file

```bash
./target/release/fitslook
```

This launches the viewer in an empty state — the UI will show "Open a FITS file to view".

---

## User Interface Overview

The app uses a three-panel layout:

```
┌──────────────┬────────────────────────┬──────────────┐
│  LEFT PANEL  │     CENTER PANEL       │  RIGHT PANEL │
│              │                        │              │
│ FILE         │   Image / Spectrum     │ VISUALIZATION│
│ STRUCTURE    │   / Table view         │  Algorithm   │
│ (HDU list)   │                        │  Colormap    │
│              │                        │  VMIN/VMAX   │
│ METADATA     │                        │              │
│ (Header)     │                        │ CUBE         │
│              │                        │  Collapse    │
│              │                        │  Frame       │
└──────────────┴────────────────────────┴──────────────┘
```

### Left Panel (280px)

- **FILE STRUCTURE**: Lists all HDUs (Header-Data Units) in the FITS file, showing index, name, type (IMG/TAB), and shape.
- **METADATA**: Displays the complete FITS header for the selected HDU.

### Center Panel

Displays the content based on the HDU type:
- **Image mode**: Rendered FITS image with colormap and normalization
- **Spectrum mode**: Interactive line plot of flux vs. wavelength (with optional error bands)
- **Table mode**: Scrollable table with columns and rows

The **info bar** at the top shows the OBJECT name and mouse coordinates.

### Right Panel (260px)

- **Algorithm**: Choose normalization — Linear (ZScale), Log, Sqrt, Asinh, Power
- **Colormap**: Choose from magma, viridis, inferno, gray, plasma
- **VMIN / VMAX**: Fine-tune the display stretch via text fields or sliders
- **CUBE**: (visible for 3D data cubes only) Toggle between individual frames and collapsed 2D projections (Sum/Mean/Max)

---

## Features

### Smart HDU Selection

When opening a file, the viewer automatically selects the best HDU:
- Prefers HDUs with `WAVEMIN` header (spectrum data)
- Falls back to the first image HDU with data

### Spectrum Detection

An HDU is treated as a spectrum if:
1. Its header contains `WAVEMIN`, OR
2. Its data is 1-dimensional, OR
3. Its data is 2D with fewer than 10 rows (flux + error format, e.g. SDSS)

When detected, the center panel shows an interactive plot with:
- **Flux line** (cyan)
- **Error band** (semi-transparent fill, if a second row exists)
- Wavelength axis built from `WAVEMIN` / `WAVEMAX` headers

### Image Normalization

| Algorithm | Description |
|-----------|-------------|
| **Linear (ZScale)** | Automatic contrast stretch using the ZScale algorithm |
| **Log** | Logarithmic scaling — good for high dynamic range data |
| **Sqrt** | Square-root scaling — emphasizes faint structures |
| **Asinh** | Inverse hyperbolic sine — excellent for nebulae |
| **Power** | Power-law (quadratic) — enhances bright features |

### Data Cube Navigation

For 3D FITS cubes (e.g. IFU data, radio cubes):
- **Frame slider**: Browse individual frames along the first axis
- **Collapse mode**: Project the entire cube into 2D using Sum, Mean, or Max

### Binary & ASCII Table Support

For table HDUs (BINTABLE or TABLE extensions), the viewer displays:
- Column headers from `TTYPEn` keywords
- Data values parsed from their binary format (`TFORMn`)
- Up to 500 rows displayed

### Mouse Tracking

Hover over image displays to see the pixel X/Y coordinates in the info bar.

---

## Linux Desktop Integration

On first launch, the app creates a `.desktop` file at:

```
~/.local/share/applications/fitslook.desktop
```

This registers FITSLook with your desktop environment so you can:
- Right-click a `.fits` file → **Open With → AstroFITS**
- Search for "AstroFITS" in your application launcher

To refresh the database manually:

```bash
update-desktop-database ~/.local/share/applications
```

---

## Troubleshooting

### Q: The app crashes with GL/display errors on Wayland

Set the backend to X11:

```bash
WINIT_UNIX_BACKEND=x11 ./target/release/fitslook
```

### Q: Build fails with linker errors about `GL` or `X11`

Install the system development libraries:

```bash
# Fedora
sudo dnf install mesa-libGL-devel mesa-libEGL-devel libxkbcommon-devel

# Ubuntu
sudo apt install libgl-dev libegl-dev libxkbcommon-dev
```

### Q: The app shows "Empty or invalid FITS file"

Ensure the file is a valid FITS file conforming to the NASA/IETF standard. Compressed FITS (`.fits.gz`) is **not** currently supported — decompress first with `gzip -d`.

### Q: Desktop integration doesn't appear

Move the binary to a persistent location (e.g. `~/.local/bin/fitslook`), then run it once:

```bash
~/.local/bin/fitslook
```

If the shortcut still doesn't appear, try:

```bash
update-desktop-database ~/.local/share/applications
```

Or restart your desktop session.

---

## Architecture (For Developers)

| File | Responsibility |
|------|---------------|
| `main.rs` | Entry point, CLI args, launches eframe window |
| `app.rs` | GUI layout + state, three-panel design, event handling |
| `fits_data.rs` | Pure-Rust FITS parser (images, tables, cubes), no C deps |
| `rendering.rs` | ZScale, normalization, colormap → RGBA pixel conversion |
| `spectrum.rs` | Spectrum detection heuristics + wavelength axis builder |
| `platform.rs` | Linux .desktop registration, file:// URI decoding |
