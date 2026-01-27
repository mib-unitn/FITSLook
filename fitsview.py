import sys, os, subprocess, numpy as np
import urllib.parse
from astropy.io import fits
from astropy.visualization import ZScaleInterval, simple_norm
from matplotlib.figure import Figure
from matplotlib.backends.backend_qtagg import FigureCanvasQTAgg as FigureCanvas
from PyQt6.QtWidgets import (QApplication, QMainWindow, QVBoxLayout, QHBoxLayout, 
                             QWidget, QLabel, QSlider, QComboBox, QTextEdit, 
                             QTableWidget, QTableWidgetItem, QStackedWidget, 
                             QRadioButton, QLineEdit, QFrame, QListWidget, QSplitter)
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont, QIcon, QColor

def resource_path(relative_path):
    base_path = getattr(sys, '_MEIPASS', os.path.abspath("."))
    return os.path.join(base_path, relative_path)

STYLE = """
QMainWindow { background-color: #0d1117; }
QFrame#GlassPanel {
    background-color: rgba(22, 27, 34, 0.95);
    border: 1px solid rgba(48, 54, 61, 1.0);
    border-radius: 12px;
}
QLabel { color: #c9d1d9; font-family: 'Segoe UI', sans-serif; font-size: 12px; font-weight: 500; }
QLabel#Title { color: #58a6ff; font-size: 14px; font-weight: bold; }
QLineEdit, QComboBox {
    background-color: #0d1117; border: 1px solid #30363d; border-radius: 6px;
    color: #58a6ff; padding: 5px; font-family: 'Consolas', monospace;
}
QListWidget { background-color: transparent; border: none; color: #8b949e; outline: none; }
QListWidget::item { padding: 8px; border-radius: 6px; margin-bottom: 4px; }
QListWidget::item:selected { background-color: rgba(88, 166, 255, 0.15); color: #58a6ff; border: 1px solid rgba(88, 166, 255, 0.3); }
QSlider::groove:horizontal { height: 4px; background: #21262d; border-radius: 2px; }
QSlider::handle:horizontal {
    background: #58a6ff; border: 2px solid #0d1117; width: 16px; height: 16px; margin: -6px 0; border-radius: 8px;
}
QTableWidget { background-color: #0d1117; gridline-color: #30363d; color: #c9d1d9; border: none; }
QHeaderView::section { background-color: #161b22; border: none; color: #8b949e; padding: 4px; }
QTextEdit { background-color: #0d1117; border: 1px solid #30363d; border-radius: 8px; color: #79c0ff; }
QSplitter::handle { background-color: #30363d; width: 1px; }
"""

class FITSViewer(QMainWindow):
    def __init__(self, filepath=None):
        super().__init__()
        self.hdul = None
        self.full_data = None
        self.is_spectrum = False
        self.block_signals = False 

        self.setWindowTitle("AstroFITS Explorer")
        self.setWindowIcon(QIcon(resource_path("app_icon.png")))
        self.resize(1400, 950)
        self.setStyleSheet(STYLE)

        main_splitter = QSplitter(Qt.Orientation.Horizontal)
        self.setCentralWidget(main_splitter)

        # LEFT PANEL
        self.left_panel = QFrame(); self.left_panel.setObjectName("GlassPanel"); self.left_panel.setFixedWidth(280)
        left_l = QVBoxLayout(self.left_panel)
        left_l.addWidget(QLabel("FILE STRUCTURE", objectName="Title"))
        self.hdu_list = QListWidget()
        self.hdu_list.currentRowChanged.connect(self.load_hdu)
        left_l.addWidget(self.hdu_list, stretch=1)
        left_l.addWidget(QLabel("METADATA", objectName="Title"))
        self.header_box = QTextEdit(); self.header_box.setReadOnly(True); self.header_box.setFont(QFont("Monospace", 9))
        left_l.addWidget(self.header_box, stretch=2)
        main_splitter.addWidget(self.left_panel)

        # CENTER PANEL
        self.center_panel = QFrame(); self.center_panel.setObjectName("GlassPanel")
        center_l = QVBoxLayout(self.center_panel); center_l.setContentsMargins(0,0,0,0)
        info_bar = QHBoxLayout(); info_bar.setContentsMargins(15, 10, 15, 0)
        self.obj_lbl = QLabel("OBJECT: --", objectName="Title"); self.obj_lbl.setStyleSheet("color: #79c0ff;")
        self.coord_lbl = QLabel("READY"); self.coord_lbl.setAlignment(Qt.AlignmentFlag.AlignRight)
        info_bar.addWidget(self.obj_lbl); info_bar.addStretch(); info_bar.addWidget(self.coord_lbl)
        center_l.addLayout(info_bar)
        
        self.stack = QStackedWidget()
        self.fig = Figure(facecolor='#161b22', tight_layout=True)
        self.ax = self.fig.add_subplot(111); self.ax.set_facecolor('#0d1117')
        self.canvas = FigureCanvas(self.fig)
        self.canvas.mpl_connect('motion_notify_event', self.on_mouse_move)
        self.stack.addWidget(self.canvas)
        self.table_view = QTableWidget()
        self.stack.addWidget(self.table_view)
        center_l.addWidget(self.stack)
        main_splitter.addWidget(self.center_panel)

        # RIGHT PANEL
        self.right_panel = QFrame(); self.right_panel.setObjectName("GlassPanel"); self.right_panel.setFixedWidth(260)
        right_l = QVBoxLayout(self.right_panel); right_l.setSpacing(20)
        
        self.img_group = QWidget()
        img_gl = QVBoxLayout(self.img_group); img_gl.setContentsMargins(0,0,0,0)
        img_gl.addWidget(QLabel("VISUALIZATION", objectName="Title"))
        self.norm_combo = QComboBox(); self.norm_combo.addItems(["Linear (ZScale)", "Log", "Sqrt", "Asinh", "Power"])
        self.norm_combo.currentIndexChanged.connect(self.update_view_logic)
        img_gl.addWidget(QLabel("Algorithm:")); img_gl.addWidget(self.norm_combo)
        self.cmap_combo = QComboBox(); self.cmap_combo.addItems(['magma', 'viridis', 'inferno', 'gray', 'plasma'])
        self.cmap_combo.currentTextChanged.connect(self.update_cmap)
        img_gl.addWidget(QLabel("Colormap:")); img_gl.addWidget(self.cmap_combo)
        
        for k in ['vmin', 'vmax']:
            row = QHBoxLayout(); lbl = QLabel(k.upper()); edit = QLineEdit(); edit.setFixedWidth(80)
            setattr(self, f"{k}_edit", edit); row.addWidget(lbl); row.addStretch(); row.addWidget(edit)
            img_gl.addLayout(row)
            s = QSlider(Qt.Orientation.Horizontal); s.setRange(0, 10000); setattr(self, f"{k}_s", s)
            s.valueChanged.connect(self.sync_sliders_to_text)
            edit.returnPressed.connect(self.sync_text_to_sliders)
            img_gl.addWidget(s)
        right_l.addWidget(self.img_group)

        self.cube_group = QWidget()
        cube_gl = QVBoxLayout(self.cube_group); cube_gl.setContentsMargins(0,0,0,0)
        cube_gl.addWidget(QLabel("CUBE", objectName="Title"))
        self.radio_integ = QRadioButton("Collapse (2D)"); self.radio_integ.setStyleSheet("color: #c9d1d9;")
        self.integ_method = QComboBox(); self.integ_method.addItems(["Sum", "Mean", "Max"])
        self.frame_slider = QSlider(Qt.Orientation.Horizontal)
        cube_gl.addWidget(self.radio_integ); cube_gl.addWidget(self.integ_method); cube_gl.addWidget(QLabel("Frame:")); cube_gl.addWidget(self.frame_slider)
        right_l.addWidget(self.cube_group); right_l.addStretch()
        
        main_splitter.addWidget(self.right_panel)
        main_splitter.setStretchFactor(1, 4)

        self.radio_integ.toggled.connect(self.update_view_logic)
        self.integ_method.currentIndexChanged.connect(self.update_view_logic)
        self.frame_slider.valueChanged.connect(self.update_view_logic)
        self.stack.currentChanged.connect(self.on_tab_changed)

        if filepath: self.open_file(filepath)

    def on_tab_changed(self, index):
        if index == 0: self.fig.tight_layout(); self.canvas.draw_idle()

    def open_file(self, filepath):
        # FIX: Robust Path Decoding for Linux URIs (e.g., file:///home/user/My%20Galaxy.fits)
        if filepath.startswith("file://"):
            filepath = urllib.parse.unquote(filepath[7:])
        
        try:
            self.hdul = fits.open(filepath)
            self.hdu_list.blockSignals(True); self.hdu_list.clear()
            idx = 0
            for i, h in enumerate(self.hdul):
                ext = h.name if h.name else "PRIMARY"
                typ = "IMG" if h.is_image else "TAB"
                shape = str(h.data.shape) if h.data is not None else "(-)"
                self.hdu_list.addItem(f"{i} | {ext}\n{typ} {shape}")
                if h.is_image and ('WAVEMIN' in h.header or (h.data is not None and h.data.ndim==2 and h.data.shape[0]<10)):
                    idx = i
            
            self.hdu_list.blockSignals(False)
            self.hdu_list.setCurrentRow(idx) # This triggers the visual update
            self.load_hdu(idx) # Force call to ensure it loads even if signals miss
            self.setWindowTitle(f"AstroFITS - {os.path.basename(filepath)}")
        except Exception as e:
            self.header_box.setPlainText(f"Error loading file:\n{str(e)}")

    def load_hdu(self, index):
        if not self.hdul: return
        hdu = self.hdul[index]
        self.header_box.setPlainText(hdu.header.tostring(sep='\n'))
        self.obj_lbl.setText(f"OBJECT: {hdu.header.get('OBJECT', 'Unknown')}")
        
        if not hdu.is_image or hdu.data is None:
            self.stack.setCurrentIndex(1); self.right_panel.hide(); self.populate_table(hdu.data); return
        
        self.stack.setCurrentIndex(0); self.right_panel.show()
        self.full_data = np.nan_to_num(hdu.data.astype(float))
        h = hdu.header
        self.is_spectrum = 'WAVEMIN' in h or (self.full_data.ndim == 2 and self.full_data.shape[0] < 10) or self.full_data.ndim == 1
        
        if self.is_spectrum:
            self.img_group.hide(); self.cube_group.hide(); self.render_spectrum(h)
        else:
            self.img_group.show(); self.cube_group.setVisible(self.full_data.ndim >= 3)
            if self.full_data.ndim >= 3: self.frame_slider.setMaximum(self.full_data.shape[0]-1)
            self.update_view_logic()

    def render_spectrum(self, h):
        self.ax.clear(); data = self.full_data
        nx = data.shape[1] if data.ndim > 1 else data.shape[0]
        wmin = h.get('WAVEMIN', 0); wmax = h.get('WAVEMAX', nx)
        wave = np.linspace(wmin, wmax, nx)
        flux = data[0] if data.ndim > 1 else data
        self.ax.plot(wave, flux, color='#00f0ff', lw=1.2, label='Flux')
        if data.ndim > 1 and data.shape[0] > 1:
            self.ax.fill_between(wave, flux - data[1], flux + data[1], alpha=0.15, color='#00f0ff')
        self.ax.set_xlim(wmin, wmax); self.ax.margins(x=0); self.ax.set_aspect('auto')
        self.ax.tick_params(colors='#8b949e', grid_alpha=0.1, grid_color='#30363d'); self.ax.grid(True)
        self.ax.set_xlabel("Wavelength", color='#c9d1d9'); self.ax.set_ylabel("Flux", color='#c9d1d9')
        self.fig.tight_layout(); self.canvas.draw()

    def update_view_logic(self):
        if self.full_data is None or getattr(self, 'is_spectrum', False): return
        if self.radio_integ.isChecked() and self.full_data.ndim >= 3:
            m = self.integ_method.currentText()
            self.current_view = np.sum(self.full_data, axis=0) if m == "Sum" else (np.mean(self.full_data, axis=0) if m == "Mean" else np.max(self.full_data, axis=0))
        else:
            idx = self.frame_slider.value() if self.full_data.ndim >= 3 else 0
            self.current_view = self.full_data[idx] if self.full_data.ndim >= 3 else self.full_data
        
        self.d_min, self.d_max = np.nanmin(self.current_view), np.nanmax(self.current_view)
        algo = self.norm_combo.currentText()
        v1, v2 = ZScaleInterval().get_limits(self.current_view) if "Linear" in algo else (self.d_min, self.d_max)
        
        self.block_signals = True
        self.vmin_edit.setText(f"{v1:.4f}"); self.vmax_edit.setText(f"{v2:.4f}")
        self.sync_text_to_sliders()
        self.block_signals = False
        self.render_image()

    def render_image(self):
        self.ax.clear(); self.ax.axis('off')
        try: v1, v2 = float(self.vmin_edit.text()), float(self.vmax_edit.text())
        except: v1, v2 = self.d_min, self.d_max
        algo = self.norm_combo.currentText()
        
        norm = None
        if "Log" in algo: norm = simple_norm(self.current_view, 'log', min_cut=v1, max_cut=v2)
        elif "Sqrt" in algo: norm = simple_norm(self.current_view, 'sqrt', min_cut=v1, max_cut=v2)
        elif "Asinh" in algo: norm = simple_norm(self.current_view, 'asinh', min_cut=v1, max_cut=v2)
        elif "Power" in algo: norm = simple_norm(self.current_view, 'power', min_cut=v1, max_cut=v2)
        
        if norm: self.img_disp = self.ax.imshow(self.current_view, cmap=self.cmap_combo.currentText(), norm=norm, origin='lower')
        else: self.img_disp = self.ax.imshow(self.current_view, cmap=self.cmap_combo.currentText(), vmin=v1, vmax=v2, origin='lower')
        self.ax.set_aspect('equal'); self.fig.tight_layout(); self.canvas.draw()

    def sync_sliders_to_text(self):
        if self.block_signals or self.full_data is None: return
        self.block_signals = True
        v1 = self.d_min + (self.vmin_s.value()/10000)*(self.d_max-self.d_min)
        v2 = self.d_min + (self.vmax_s.value()/10000)*(self.d_max-self.d_min)
        self.vmin_edit.setText(f"{v1:.4f}"); self.vmax_edit.setText(f"{v2:.4f}")
        self.render_image(); self.block_signals = False

    def sync_text_to_sliders(self):
        if self.full_data is None: return
        try:
            v1, v2 = float(self.vmin_edit.text()), float(self.vmax_edit.text())
            den = (self.d_max - self.d_min) if self.d_max != self.d_min else 1
            self.vmin_s.setValue(int(((v1-self.d_min)/den)*10000))
            self.vmax_s.setValue(int(((v2-self.d_min)/den)*10000))
            self.render_image()
        except: pass

    def on_mouse_move(self, event):
        if event.inaxes == self.ax: self.coord_lbl.setText(f"X: {event.xdata:.1f}  Y: {event.ydata:.1f}")

    def update_cmap(self, n):
        if hasattr(self, 'img_disp'): self.img_disp.set_cmap(n); self.canvas.draw_idle()

    def populate_table(self, data):
        if data is None: return
        self.table_view.setRowCount(min(len(data), 200)); self.table_view.setColumnCount(len(data.columns))
        self.table_view.setHorizontalHeaderLabels(data.columns.names)
        for i in range(min(len(data), 200)):
            for j in range(len(data.columns)):
                item = QTableWidgetItem(str(data[i][j])); item.setForeground(QColor("#79c0ff"))
                self.table_view.setItem(i, j, item)

def ensure_linux_integration():
    if not sys.platform.startswith('linux'): return
    exe = os.path.realpath(sys.executable if getattr(sys, 'frozen', False) else sys.argv[0])
    desktop = os.path.expanduser("~/.local/share/applications/fitslook.desktop")
    icon = resource_path("app_icon.png")
    c = f"[Desktop Entry]\nName=AstroFITS\nExec=\"{exe}\" %u\nType=Application\nIcon={icon}\nMimeType=image/fits;image/x-fits;application/fits;"
    with open(desktop, 'w') as f: f.write(c)
    subprocess.run(["update-desktop-database", os.path.expanduser("~/.local/share/applications")])

if __name__ == "__main__":
    ensure_linux_integration(); app = QApplication(sys.argv)
    window = FITSViewer(sys.argv[1] if len(sys.argv) > 1 else None)
    window.show(); sys.exit(app.exec())