# FITSLook 🔭

**The missing Quick Look plugin for Astronomers.**

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)

FITSLook is a lightweight utility that brings native preview capabilities to FITS (Flexible Image Transport System) files. Stop opening heavy image processing software just to check if a frame is good—preview headers and image data directly from your file explorer.

## ✨ Features

* **Instant Preview:** View FITS images immediately in Finder (macOS) or your file explorer without launching full-scale applications like DS9 or PixInsight.
* **Header Inspection:** Scroll through the FITS Header Data Unit (HDU) to verify keywords, exposure time, object name, and telescope telemetry.
* **Automatic Stretching:** Applies intelligent auto-stretch (ZScale or Min/Max) to make faint astronomical data visible immediately.
* **Lightweight:** Optimized for speed; minimal memory footprint.

## 🚀 Installation (Pre-built)

### Option 1: Homebrew

Coming soon...

### Option 2: Manual Install
Download the latest release from the Releases Page.

Unzip FITSLook.app.

Move it to your /Applications folder.

Run the app once to register the Quick Look service.


## 📖 Usage
Using Quick Look (macOS)
Select a .fits or .fit file in Finder.

Press Spacebar.

You will see a rendered preview of the image data alongside the primary header metadata.

Using the Standalone Viewer
Double-click any FITS file to open it in the dedicated FITSLook window for a slightly more detailed view, including histogram checks and basic zoom.

## 🤝 Contributing
Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are greatly appreciated.

Fork the Project

Create your Feature Branch (git checkout -b feature/AmazingFeature)

Commit your Changes (git commit -m 'Add some AmazingFeature')

Push to the Branch (git push origin feature/AmazingFeature)

Open a Pull Request

## 📜 License
Distributed under the MIT License. See LICENSE for more information.

## Acknowledgements
Built using CFITSIO

Inspired by the needs of astrophotographers worldwide.


---

### Next Step for you

Would you like me to generate a **`.gitignore` file** tailored for this project? I can ensure it correctly ignores the Xcode build artifacts and the `.DS_Store` files while ensuring your `app_icon.png` is tracked.
