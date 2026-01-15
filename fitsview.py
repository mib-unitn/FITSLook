import sys, os, subprocess, numpy as np
from astropy.io import fits
from astropy.visualization import ZScaleInterval
from matplotlib.figure import Figure
from matplotlib.backends.backend_qtagg import FigureCanvasQTAgg as FigureCanvas
from PyQt6.QtWidgets import (QApplication, QMainWindow, QVBoxLayout, QHBoxLayout, 
                             QWidget, QLabel, QSlider, QComboBox, QTextEdit, 
                             QTableWidget, QTableWidgetItem, QStackedWidget, 
                             QRadioButton, QLineEdit, QFrame, QGroupBox)
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont, QIcon, QColor

def resource_path(relative_path):
    base_path = getattr(sys, '_MEIPASS', os.path.abspath("."))
    return os.path.join(base_path, relative_path)

# --- APPLE LIQUID GLASS LIGHT STYLING ---
STYLE = """
QMainWindow { background-color: #f5f5f7; }
QFrame#GlassPanel {
    background-color: rgba(255, 255, 255, 180);
    border: 1px solid rgba(210, 210, 215, 150);
    border-radius: 18px;
}
QGroupBox {
    font-weight: bold;
    color: #1d1d1f;
    border: 1px solid rgba(0, 0, 0, 20);
    border-radius: 10px;
    margin-top: 15px;
    background-color: rgba(255, 255, 255, 80);
}
QGroupBox::title { subcontrol-origin: margin; left: 10px; padding: 0 5px; }
QLabel { color: #1d1d1f; font-family: 'SF Pro Display', 'Inter', sans-serif; font-size: 13px; }
QLineEdit, QComboBox {
    background-color: white;
    border: 1px solid #d2d2d7;
    border-radius: 6px;
    color: #1d1d1f;
    padding: 4px;
}
QLineEdit:focus, QComboBox:hover { border: 1.5px solid #007aff; }
QSlider::groove:horizontal { height: 6px; background: #e5e5ea; border-radius: 3px; }
QSlider::handle:horizontal {
    background: white; border: 1px solid #d2d2d7;
    width: 20px; height: 20px; margin: -8px 0; border-radius: 10px;
}
QTableWidget {
    background-color: white; alternate-background-color: #fafafa;
    gridline-color: #e5e5ea; color: #1d1d1f; border-radius: 10px;
}
QHeaderView::section { background-color: #f5f5f7; border: none; font-weight: bold; }
QTextEdit { background-color: rgba(255,255,255,100); border: 1px solid #d2d2d7; border-radius: 10px; color: #3a3a3c; }
"""

class FITSViewer(QMainWindow):
    def __init__(self, filepath=None):
        super().__init__()
        # 1. State Init
        self.hdul = None
        self.full_data = None
        self.is_spectrum = False
        self.block_signals = False 

        # 2. Window Setup
        self.setWindowTitle("FITSLook Pro")
        self.setWindowIcon(QIcon(resource_path("app_icon.png")))
        self.resize(1200, 950)
        self.setStyleSheet(STYLE)

        container = QWidget()
        self.setCentralWidget(container)
        main_layout = QVBoxLayout(container)
        main_layout.setContentsMargins(20, 20, 20, 20)
        main_layout.setSpacing(10)

        # Top Extension Bar
        self.top_bar = QFrame(); self.top_bar.setObjectName("GlassPanel")
        top_l = QHBoxLayout(self.top_bar)
        self.hdu_combo = QComboBox()
        self.hdu_combo.currentIndexChanged.connect(self.load_hdu)
        self.coord_lbl = QLabel("RA/Dec: Ready")
        self.coord_lbl.setStyleSheet("color: #007aff; font-weight: bold;")
        top_l.addWidget(QLabel("<b>Extension:</b>")); top_l.addWidget(self.hdu_combo)
        top_l.addStretch(); top_l.addWidget(self.coord_lbl)
        main_layout.addWidget(self.top_bar)

        # Plot/Table Display
        self.disp_frame = QFrame(); self.disp_frame.setObjectName("GlassPanel")
        disp_l = QVBoxLayout(self.disp_frame)
        self.stack = QStackedWidget()
        self.fig = Figure(facecolor='none', tight_layout=True)
        self.ax = self.fig.add_subplot(111)
        self.canvas = FigureCanvas(self.fig)
        self.canvas.mpl_connect('motion_notify_event', self.on_mouse_move)
        self.stack.addWidget(self.canvas)
        self.table_view = QTableWidget(); self.table_view.setAlternatingRowColors(True)
        self.stack.addWidget(self.table_view)
        disp_l.addWidget(self.stack)
        main_layout.addWidget(self.disp_frame, stretch=10)

        # Controls Container
        self.ctrl_area = QFrame(); self.ctrl_area.setObjectName("GlassPanel")
        ctrl_l = QVBoxLayout(self.ctrl_area)

        # Image Logic
        self.img_group = QGroupBox("Contrast & Controls")
        img_layout = QVBoxLayout(self.img_group)
        
        # Colormap row
        c_row = QHBoxLayout()
        self.cmap_combo = QComboBox(); self.cmap_combo.addItems(['magma', 'viridis', 'gray', 'plasma'])
        self.cmap_combo.currentTextChanged.connect(self.update_cmap)
        c_row.addWidget(QLabel("Colormap:")); c_row.addWidget(self.cmap_combo)
        self.radio_integ = QRadioButton("Integrate Cube")
        self.integ_method = QComboBox(); self.integ_method.addItems(["Sum", "Mean", "Max"])
        c_row.addStretch(); c_row.addWidget(self.radio_integ); c_row.addWidget(self.integ_method)
        img_layout.addLayout(c_row)

        # Frame Slider
        self.frame_slider = QSlider(Qt.Orientation.Horizontal)
        img_layout.addWidget(self.frame_slider)

        # VMIN/VMAX Boxes + Sliders
        for k in ['vmin', 'vmax']:
            row = QHBoxLayout()
            e = QLineEdit(); e.setFixedWidth(110); setattr(self, f"{k}_edit", e)
            s = QSlider(Qt.Orientation.Horizontal); s.setRange(0, 10000); setattr(self, f"{k}_s", s)
            row.addWidget(QLabel(f"{k.upper()}:")); row.addWidget(e); row.addWidget(s)
            img_layout.addLayout(row)
            s.valueChanged.connect(self.sync_sliders_to_text); e.returnPressed.connect(self.sync_text_to_sliders)

        ctrl_l.addWidget(self.img_group)
        main_layout.addWidget(self.ctrl_area)

        # Bottom Header Info
        self.header_box = QTextEdit(); self.header_box.setReadOnly(True); self.header_box.setFont(QFont("Monospace", 9))
        self.header_box.setMaximumHeight(140); main_layout.addWidget(self.header_box)

        # Final connections
        self.radio_integ.toggled.connect(self.update_view_logic)
        self.integ_method.currentIndexChanged.connect(self.update_view_logic)
        self.frame_slider.valueChanged.connect(self.update_view_logic)

        if filepath: self.open_file(filepath)

    def open_file(self, filepath):
        if filepath.startswith('file://'): filepath = filepath[7:]
        try:
            self.hdul = fits.open(filepath)
            self.hdu_combo.blockSignals(True); self.hdu_combo.clear()
            for i, h in enumerate(self.hdul): self.hdu_combo.addItem(f"HDU {i}: {h.name}")
            idx = 4 if len(self.hdul) > 4 and 'WAVEMIN' in self.hdul[4].header else 0
            self.hdu_combo.blockSignals(False); self.hdu_combo.setCurrentIndex(idx); self.load_hdu(idx)
        except Exception as e: self.header_box.setPlainText(str(e))

    def load_hdu(self, index):
        if not self.hdul: return
        hdu = self.hdul[index]
        self.header_box.setPlainText(hdu.header.tostring(sep='\n'))
        if not hdu.is_image or hdu.data is None:
            self.stack.setCurrentIndex(1); self.ctrl_area.hide(); self.populate_table(hdu.data); return
        
        self.stack.setCurrentIndex(0); self.ctrl_area.show()
        self.full_data = np.nan_to_num(hdu.data.astype(float))
        h = hdu.header
        
        # Smart Detection for 1D Spectra
        self.is_spectrum = 'WAVEMIN' in h or (self.full_data.ndim == 2 and self.full_data.shape[0] < 10)
        
        if self.is_spectrum:
            self.img_group.hide(); self.render_spectrum(h)
        else:
            self.img_group.show(); self.frame_slider.setVisible(self.full_data.ndim >= 3)
            if self.full_data.ndim >= 3: self.frame_slider.setMaximum(self.full_data.shape[0]-1)
            self.update_view_logic()

    def render_spectrum(self, h):
        self.ax.clear(); self.ax.set_facecolor('none')
        data = self.full_data
        nx = data.shape[1] if data.ndim > 1 else data.shape[0]
        
        # Proper scaling of x-axis limits
        wmin = h.get('WAVEMIN', 0); wmax = h.get('WAVEMAX', nx)
        wave = np.linspace(wmin, wmax, nx)
        
        flux = data[0] if data.ndim > 1 else data
        self.ax.plot(wave, flux, color='#007aff', lw=1.2, label='Flux')
        if data.ndim > 1 and data.shape[0] > 1:
            self.ax.fill_between(wave, flux - data[1], flux + data[1], alpha=0.2, color='#007aff')

        # Fit Spectrum to Full Window: Remove horizontal margins
        self.ax.set_xlim(wmin, wmax)
        self.ax.margins(x=0)
        
        self.ax.tick_params(colors='#1d1d1f'); self.ax.grid(True, alpha=0.1, color='#8e8e93')
        self.ax.set_xlabel(f"Wavelength ({h.get('CUNIT1', 'Å')})"); self.ax.set_ylabel("Flux")
        self.fig.tight_layout()
        self.canvas.draw()

    def update_view_logic(self):
        if self.full_data is None or getattr(self, 'is_spectrum', False): return
        if self.radio_integ.isChecked() and self.full_data.ndim >= 3:
            m = self.integ_method.currentText()
            self.current_view = np.sum(self.full_data, axis=0) if m == "Sum" else (np.mean(self.full_data, axis=0) if m == "Mean" else np.max(self.full_data, axis=0))
        else:
            idx = self.frame_slider.value() if self.full_data.ndim >= 3 else 0
            self.current_view = self.full_data[idx] if self.full_data.ndim >= 3 else self.full_data
        
        self.d_min, self.d_max = np.nanmin(self.current_view), np.nanmax(self.current_view)
        v1, v2 = ZScaleInterval().get_limits(self.current_view)
        self.block_signals = True; self.vmin_edit.setText(f"{v1:.4f}"); self.vmax_edit.setText(f"{v2:.4f}"); self.sync_text_to_sliders(); self.block_signals = False
        self.render_image()

    def render_image(self):
        self.ax.clear(); self.ax.axis('off')
        try: v1, v2 = float(self.vmin_edit.text()), float(self.vmax_edit.text())
        except: v1, v2 = 0, 1
        self.img_disp = self.ax.imshow(self.current_view, cmap=self.cmap_combo.currentText(), vmin=v1, vmax=v2, origin='lower')
        self.canvas.draw()

    def sync_sliders_to_text(self):
        if self.block_signals or self.full_data is None: return
        self.block_signals = True
        v1 = self.d_min + (self.vmin_s.value()/10000)*(self.d_max-self.d_min)
        v2 = self.d_min + (self.vmax_s.value()/10000)*(self.d_max-self.d_min)
        self.vmin_edit.setText(f"{v1:.4f}"); self.vmax_edit.setText(f"{v2:.4f}")
        if hasattr(self, 'img_disp'): self.img_disp.set_clim(v1, v2 if v2 > v1 else v1+1e-5)
        self.canvas.draw_idle(); self.block_signals = False

    def sync_text_to_sliders(self):
        if self.full_data is None: return
        try:
            v1, v2 = float(self.vmin_edit.text()), float(self.vmax_edit.text())
            den = (self.d_max - self.d_min) if self.d_max != self.d_min else 1
            self.vmin_s.setValue(int(((v1-self.d_min)/den)*10000))
            self.vmax_s.setValue(int(((v2-self.d_min)/den)*10000))
            if hasattr(self, 'img_disp'): self.img_disp.set_clim(v1, v2); self.canvas.draw_idle()
        except: pass

    def on_mouse_move(self, event):
        if event.inaxes == self.ax: self.coord_lbl.setText(f"X: {event.xdata:.1f} Y: {event.ydata:.2e}")

    def update_cmap(self, n):
        if hasattr(self, 'img_disp'): self.img_disp.set_cmap(n); self.canvas.draw_idle()

    def update_frame(self, val):
        if not self.radio_integ.isChecked(): self.update_view_logic()

    def populate_table(self, data):
        if data is None: return
        self.table_view.setRowCount(min(len(data), 200))
        self.table_view.setColumnCount(len(data.columns))
        self.table_view.setHorizontalHeaderLabels(data.columns.names)
        for i in range(min(len(data), 200)):
            for j in range(len(data.columns)):
                item = QTableWidgetItem(str(data[i][j]))
                item.setForeground(QColor("#1d1d1f"))
                self.table_view.setItem(i, j, item)

def ensure_linux_integration():
    if not sys.platform.startswith('linux'): return
    exe = os.path.realpath(sys.executable if getattr(sys, 'frozen', False) else sys.argv[0])
    desktop = os.path.expanduser("~/.local/share/applications/fitslook.desktop")
    icon = resource_path("app_icon.png")
    c = f"[Desktop Entry]\nName=FITSLook Pro\nExec=\"{exe}\" %u\nType=Application\nIcon={icon}\nMimeType=image/fits;image/x-fits;application/fits;"
    with open(desktop, 'w') as f: f.write(c)
    subprocess.run(["update-desktop-database", os.path.expanduser("~/.local/share/applications")])

if __name__ == "__main__":
    ensure_linux_integration(); app = QApplication(sys.argv)
    window = FITSViewer(sys.argv[1] if len(sys.argv)>1 else None)
    window.show(); sys.exit(app.exec())
