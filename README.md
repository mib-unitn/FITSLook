# FITSLook Pro 🔭

A professional, high-performance "Quick Look" viewer for astronomical FITS files. Designed for Linux (Fedora/Ubuntu) and macOS with a modern "Liquid Glass" interface.

![FITSLook UI](https://via.placeholder.com/800x500.png?text=FITSLook+Pro+Interface)

## ✨ Features

*   **Smart Spectrum Detection**: Automatically detects 1D arrays or 2D Flux/Error tables (including SDSS formats) and plots them as interactive spectra using header WCS data.
*   **Cube Integration**: Instantly switch between slicing 3D cubes and viewing Integrated (Sum/Mean/Max) projections.
*   **Native Integration**: Adds "Open With" shortcuts to **Nautilus**, **Dolphin**, **Thunar** (Linux), and **Finder** (macOS).
*   **Liquid Glass UI**: A modern, light-themed interface with translucent panels and Apple-style aesthetics.
*   **Zero-Lag Contrast**: High-performance sliders and editable value boxes for ZScale stretching.
*   **Metadata Viewer**: Instant access to FITS headers.

---

## 🚀 Quick Install (For Users)

### 🐧 Linux (Fedora, Ubuntu, Arch)
1.  Download the **binary** from the Releases page.
2.  Move it to a permanent location (e.g., `~/bin/`):
    ```bash
    mkdir -p ~/bin
    mv FITSLook ~/bin/
    chmod +x ~/bin/FITSLook
    ```
3.  **Run it once** from the terminal to register the desktop shortcut:
    ```bash
    ~/bin/FITSLook
    ```
4.  You can now right-click any `.fits` file → **Open With FITSLook Pro**.

### 🍎 macOS
1.  Download `FITSLook.app.zip` from Releases.
2.  Drag `FITSLook.app` into your `/Applications` folder.
3.  Right-click a FITS file → **Open With** → **FITSLook Pro**.

---

## 🛠 Building from Source (Developer Guide)

Because this app uses complex GUI libraries (Qt6) and scientific stacks (Astropy), **strict build isolation is required** to avoid crashing on Linux, especially if using Conda.

### ⚠️ Prerequisite: The "Clean Build" Rule
**DO NOT** build this using your base Conda environment (e.g., `(base)` or `(generic)`). Conda injects system variables that break PyInstaller. You must use a standard Python `venv`.

### 1. Set up the Environment (Linux & Mac)
Run these commands in your terminal to create a fresh build environment:

```bash
# 1. Deactivate any existing conda environment
conda deactivate

# 2. Create a standard virtual environment
python3 -m venv build_env

# 3. Activate it
source build_env/bin/activate

# 4. Install dependencies 
# (We force specific PyQt versions to avoid private API errors on Linux)
pip install --upgrade pip
pip install pyinstaller astropy matplotlib numpy pillow
pip install "PyQt6==6.7.1" "PyQt6-Qt6==6.7.2"
```

### 2. Build for Linux

```bash
pyinstaller --onefile --windowed --name FITSLook \
--add-data "app_icon.png:." \
--hidden-import "numpy._core._exceptions" \
--exclude-module "PyQt6.QtQml" \
--exclude-module "PyQt6.QtQuick" \
--exclude-module "PyQt6.QtSql" \
--exclude-module "PyQt6.Qt3DCore" \
--exclude-module "PyQt6.Qt3DRender" \
--exclude-module "PyQt6.QtWebEngineCore" \
--exclude-module "PyQt6.QtDesigner" \
--exclude-module "PyQt6.QtBluetooth" \
--exclude-module "PyQt6.QtMultimedia" \
--exclude-module "PyQt6.QtNfc" \
--exclude-module "PyQt6.QtPositioning" \
--exclude-module "PyQt6.QtRemoteObjects" \
--exclude-module "PyQt6.QtSensors" \
--exclude-module "PyQt6.QtSerialPort" \
--exclude-module "PyQt6.QtStateMachine" \
--exclude-module "PyQt6.QtSvg" \
--exclude-module "PyQt6.QtTest" \
--exclude-module "PyQt6.QtWebChannel" \
--exclude-module "PyQt6.QtWebSockets" \
--exclude-module "PyQt6.QtXml" \
--exclude-module "asdf" \
--exclude-module "PySide6" \
--exclude-module "PySide2" \
--exclude-module "PyQt5" \
fitsview.py
```

The binary will be located in dist/FITSLook.

### 3. Build for macOS

macOS requires an Info.plist file to handle file associations in Finder.
Create info.plist in the project folder with this content:

```xml 
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>FITS Image</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Owner</string>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>fits</string>
                <string>fit</string>
                <string>fts</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
```

Run the Build Command:

```bash
pyinstaller --onefile --windowed --name "FITSLook" \
--osx-bundle-identifier "com.astro.fitslook" \
--add-data "app_icon.png:." \
--icon "app_icon.icns" \
--extras-bundle-info info.plist \
--exclude-module asdf \
--exclude-module PySide6 \
--hidden-import numpy._core._exceptions \
fitsview.py
```

The app bundle will be located in dist/FITSLook.app.

### ❓ Troubleshooting
#### Q: I get ImportError: PyQt6 or "Failed to load platform plugin 'xcb'" on Linux.
A: Fedora and Ubuntu require system-level GL libraries. Run this:

```bash
sudo dnf install libxkbcommon-x11 xcb-util-wm xcb-util-image libglvnd-glx
```
#### Q: The app crashes immediately on launch (Linux).
A: Ensure you are running the binary from a terminal ./dist/FITSLook to see the error.
If it mentions numpy._core, ensure you added the --hidden-import "numpy._core._exceptions" flag in the build command.
If it mentions Qt_6.10_PRIVATE_API, ensure you ran the pip install "PyQt6==6.7.1" command in a clean venv as described above.
#### Q: Right-click "Open With" isn't working on Linux.
A: The app registers itself when run.
Move the binary to its final location (e.g., ~/bin).
Run it manually once: ~/bin/FITSLook.
If it still doesn't appear, try restarting the GNOME shell (Alt+F2, type r) or run update-desktop-database ~/.local/share/applications.
