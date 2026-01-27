# -*- mode: python ; coding: utf-8 -*-


a = Analysis(
    ['fitsview.py'],
    pathex=[],
    binaries=[],
    datas=[('app_icon.png', '.')],
    hiddenimports=['numpy._core._exceptions'],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=['PyQt6.QtQml', 'PyQt6.QtQuick', 'PyQt6.QtSql', 'PyQt6.Qt3DCore', 'PyQt6.Qt3DRender', 'PyQt6.Qt3DInput', 'PyQt6.Qt3DLogic', 'PyQt6.Qt3DExtras', 'PyQt6.Qt3DAnimation', 'PyQt6.QtWebEngineCore', 'PyQt6.QtDesigner', 'PyQt6.QtBluetooth', 'PyQt6.QtMultimedia', 'PyQt6.QtNfc', 'PyQt6.QtPositioning', 'PyQt6.QtRemoteObjects', 'PyQt6.QtSensors', 'PyQt6.QtSerialPort', 'PyQt6.QtStateMachine', 'PyQt6.QtSvg', 'PyQt6.QtTest', 'PyQt6.QtWebChannel', 'PyQt6.QtWebSockets', 'PyQt6.QtXml', 'asdf', 'PySide6', 'PySide2', 'PyQt5'],
    noarchive=False,
    optimize=0,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='FITSLook',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)
