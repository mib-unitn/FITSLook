import os
import sys
import subprocess
import platform

def create_default_icon():
    if not os.path.exists("app_icon.png"):
        print(" Generating default icon...")
        from PIL import Image, ImageDraw
        img = Image.new('RGBA', (256, 256), (15, 17, 26, 255))
        d = ImageDraw.Draw(img)
        d.ellipse((28, 28, 228, 228), outline=(88, 166, 255), width=10)
        d.text((80, 110), "FITS", fill=(255, 255, 255))
        img.save("app_icon.png")

def install_dependencies():
    print(" Installing dependencies...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", 
                           "pyinstaller", "astropy", "matplotlib", "PyQt6", "numpy", "pillow"])

def build_linux():
    print(" Building for Linux...")
    cmd = [
        "pyinstaller", "--onefile", "--windowed", "--name", "FITSLook",
        "--add-data", "app_icon.png:.",
        "--exclude-module", "asdf", "--exclude-module", "PySide6", "--exclude-module", "PyQt5",
        "--hidden-import", "numpy._core._exceptions",
        "fitsview.py"
    ]
    subprocess.run(cmd)
    print("\n[SUCCESS] Binary created in dist/FITSLook")
    print("Run './dist/FITSLook' once to register it with your file manager.")

def build_macos():
    print(" Building for macOS...")
    
    # Create Info.plist for Finder Integration
    plist_content = """<?xml version="1.0" encoding="UTF-8"?>
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
</plist>"""
    
    with open("info.plist", "w") as f:
        f.write(plist_content)

    cmd = [
        "pyinstaller", "--onefile", "--windowed", "--name", "FITSLook",
        "--osx-bundle-identifier", "com.astro.fitslook",
        "--add-data", "app_icon.png:.",
        "--extras-bundle-info", "info.plist",
        "--exclude-module", "asdf", "--exclude-module", "PySide6",
        "--hidden-import", "numpy._core._exceptions",
        "fitsview.py"
    ]
    subprocess.run(cmd)
    print("\n[SUCCESS] App Bundle created in dist/FITSLook.app")
    print("Drag 'dist/FITSLook.app' to your /Applications folder.")

if __name__ == "__main__":
    create_default_icon()
    # install_dependencies() # Uncomment if you want to force install
    
    if platform.system() == "Linux":
        build_linux()
    elif platform.system() == "Darwin":
        build_macos()
    else:
        print("Unsupported OS")