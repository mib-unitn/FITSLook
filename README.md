# FITSLook 🔭

A modern, high-performance "Quick Look" viewer for astronomical FITS files. Designed for macOS and Linux with a native "Liquid Glass" UI.

![Screenshot](https://via.placeholder.com/800x500.png?text=FITSLook+Pro+UI)

## Features
*   **Automatic 1D Spectrum Detection:** Smartly detects spectral data and plots it instantly.
*   **Cube Integration:** Switch between slice viewing and Sum/Mean/Max integration for 3D cubes.
*   **Modern UI:** Beautiful light-themed interface with glass-morphism panels.
*   **Native Integration:** Adds "Open With" shortcuts to Finder (macOS), Nautilus, Dolphin, and Thunar (Linux).
*   **Metadata:** Instant access to FITS headers and binary tables.

## 🚀 Installation (The Easy Way)

### 🐧 Linux (Fedora, Ubuntu, Arch, etc.)
1.  **Download:** Grab the binary from the [Releases Page](#).
2.  **Install:**
    ```bash
    mkdir -p ~/bin
    mv FITSLook ~/bin/
    chmod +x ~/bin/FITSLook
    ~/bin/FITSLook  # Run once to register right-click menu
    ```
3.  **Use:** Right-click any `.fits` file -> Open With -> FITSLook Pro.

### 🍎 macOS
1.  **Download:** Grab the `FITSLook.app` zip from the [Releases Page](#).
2.  **Install:** Drag `FITSLook.app` into your `/Applications` folder.
3.  **Use:** Right-click a FITS file -> Open With -> FITSLook.

---

## 🛠 Building from Source

If you want to modify the code or build it yourself:

### Prerequisites
*   Python 3.9+
*   `pip`

### Build Command
We provide a build script that handles everything (dependencies, icon generation, and OS integration).

1.  **Clone the repo:**
    ```bash
    git clone https://github.com/yourusername/fitslook.git
    cd fitslook
    ```

2.  **Install requirements:**
    ```bash
    pip install pyinstaller astropy matplotlib PyQt6 numpy pillow
    ```

3.  **Run the Builder:**
    ```bash
    python3 build.py
    ```

4.  **Find your App:**
    *   **Linux:** Executable is in `dist/FITSLook`
    *   **macOS:** App bundle is in `dist/FITSLook.app`
