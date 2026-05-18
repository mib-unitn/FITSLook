# FITSLook 🔭

A professional, high-performance FITS file viewer for astronomy — written in Rust with the `egui` GUI framework.

## ✨ Features

*   **Smart Spectrum Detection**: Automatically detects 1D arrays or 2D Flux/Error tables (including SDSS formats) and plots them as interactive spectra using header WCS data.
*   **Cube Integration**: Instantly switch between slicing 3D cubes and viewing Integrated (Sum/Mean/Max) projections.
*   **Native Integration**: Adds "Open With" shortcuts to **Nautilus**, **Dolphin**, **Thunar** (Linux), and **Finder** (macOS).
*   **Modern UI**: A clean, three-panel interface built with `egui`.
*   **Zero-Lag Contrast**: High-performance sliders and editable value boxes for ZScale stretching.
*   **Metadata Viewer**: Instant access to FITS headers.
*   **Table Support**: Binary and ASCII FITS table display.

---

## 🚀 Quick Install

### 🐧 Linux (Fedora, Ubuntu, Arch)

```bash
make install
```

This builds the release binary and installs everything to `~/.local/` (binary, icon, desktop entry, MIME type). See [MANUAL.md](MANUAL.md) for details.

### 🍎 macOS

```bash
cargo build --release
./install.sh
```

This creates `/Applications/AstroFITS Explorer.app` with full Finder integration.

---

## 🛠 Building from Source

### Prerequisites

**Rust Toolchain** (1.75+):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**System Dependencies (Linux)**:

```bash
# Fedora/RHEL
sudo dnf install gcc pkg-config libxkbcommon-devel wayland-devel libXcursor-devel \
    libXrandr-devel libXi-devel mesa-libGL-devel mesa-libEGL-devel

# Ubuntu/Debian
sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
    libxcursor-dev libxrandr-dev libxi-dev libgl-dev libegl-dev
```

### Build

```bash
# Debug build
cargo build

# Release build (optimized, stripped — recommended)
cargo build --release
```

The binary will be at `target/release/fitslook`.

---

## 📖 Documentation

See [MANUAL.md](MANUAL.md) for the full user manual, including UI overview, feature documentation, and troubleshooting.

---

## 🏗 Architecture

| File | Responsibility |
|------|---------------|
| `main.rs` | Entry point, CLI args, launches eframe window |
| `app.rs` | GUI layout + state, three-panel design, event handling |
| `fits_data.rs` | Pure-Rust FITS parser (images, tables, cubes), no C deps |
| `rendering.rs` | ZScale, normalization, colormap → RGBA pixel conversion |
| `spectrum.rs` | Spectrum detection heuristics + wavelength axis builder |
| `platform.rs` | Linux .desktop registration, file:// URI decoding |

---

## 📜 License

[GPL-3.0](LICENSE)
