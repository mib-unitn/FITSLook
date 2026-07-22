use eframe::egui;
use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, ScrollArea, Stroke,
    TextureHandle, TextureOptions, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints, Polygon};
use ndarray::Array2;

use crate::fits_data::{FitsDocument, ImageData};
use crate::rendering::{self, COLORMAP_NAMES};
use crate::spectrum;

#[derive(Clone, Copy)]
struct ThemePalette {
    bg_main: Color32,
    bg_panel: Color32,
    border: Color32,
    text_primary: Color32,
    text_dim: Color32,
    accent: Color32,
    accent_secondary: Color32,
    spectrum_line: Color32,
    button_fill: Color32,
    widget_inactive_fill: Color32,
    widget_hovered_fill: Color32,
    widget_active_fill: Color32,
    selection_fill: Color32,
}

const DARK_PALETTE: ThemePalette = ThemePalette {
    bg_main: Color32::from_rgb(13, 17, 23),
    bg_panel: Color32::from_rgb(22, 27, 34),
    border: Color32::from_rgb(48, 54, 61),
    text_primary: Color32::from_rgb(201, 209, 217),
    text_dim: Color32::from_rgb(139, 148, 158),
    accent: Color32::from_rgb(88, 166, 255),
    accent_secondary: Color32::from_rgb(121, 192, 255),
    spectrum_line: Color32::from_rgb(0, 240, 255),
    button_fill: Color32::from_rgb(30, 37, 48),
    widget_inactive_fill: Color32::from_rgb(13, 17, 23),
    widget_hovered_fill: Color32::from_rgb(30, 37, 48),
    widget_active_fill: Color32::from_rgb(40, 50, 65),
    selection_fill: Color32::from_rgba_premultiplied(88, 166, 255, 40),
};

// Warm ivory + giallorosso accents (AS Roma-inspired light theme).
const ROMA_LIGHT_PALETTE: ThemePalette = ThemePalette {
    bg_main: Color32::from_rgb(250, 245, 236),
    bg_panel: Color32::from_rgb(255, 251, 244),
    border: Color32::from_rgb(216, 195, 158),
    text_primary: Color32::from_rgb(52, 37, 28),
    text_dim: Color32::from_rgb(123, 96, 74),
    accent: Color32::from_rgb(133, 31, 49),
    accent_secondary: Color32::from_rgb(180, 130, 24),
    spectrum_line: Color32::from_rgb(133, 31, 49),
    button_fill: Color32::from_rgb(246, 235, 219),
    widget_inactive_fill: Color32::from_rgb(244, 234, 218),
    widget_hovered_fill: Color32::from_rgb(236, 219, 196),
    widget_active_fill: Color32::from_rgb(226, 204, 175),
    selection_fill: Color32::from_rgba_premultiplied(133, 31, 49, 38),
};

#[derive(Clone, Copy, PartialEq)]
enum UiTheme {
    Dark,
    RomaLight,
}

impl UiTheme {
    fn palette(self) -> ThemePalette {
        match self {
            UiTheme::Dark => DARK_PALETTE,
            UiTheme::RomaLight => ROMA_LIGHT_PALETTE,
        }
    }
}

// ── Normalization algorithms ─────────────────────────────────────────────
const NORM_NAMES: [&str; 5] = ["Linear (ZScale)", "Log", "Sqrt", "Asinh", "Power"];

fn algo_key(name: &str) -> &str {
    match name {
        "Linear (ZScale)" => "Linear",
        "Log" => "Log",
        "Sqrt" => "Sqrt",
        "Asinh" => "Asinh",
        "Power" => "Power",
        _ => "Linear",
    }
}

// ── Integration method ───────────────────────────────────────────────────
const INTEG_METHODS: [&str; 3] = ["Sum", "Mean", "Max"];

/// View mode for the center panel.
#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Image,
    Spectrum,
    Table,
}

/// The main application state.
pub struct FitsViewerApp {
    // File
    fits_doc: Option<FitsDocument>,
    filepath: Option<String>,

    // HDU selection
    selected_hdu: usize,

    // Current 2D view (after slicing / collapsing)
    current_view: Option<Array2<f64>>,
    view_mode: ViewMode,

    // Image rendering
    texture: Option<TextureHandle>,
    needs_rerender: bool,

    // Controls
    norm_index: usize,
    cmap_index: usize,
    vmin: f64,
    vmax: f64,
    data_min: f64,
    data_max: f64,
    vmin_slider: f32,
    vmax_slider: f32,

    // Cube
    is_cube: bool,
    collapse_mode: bool,
    integ_index: usize,
    frame_index: usize,
    frame_count: usize,

    // Spectrum data
    wavelengths: Vec<f64>,
    flux: Vec<f64>,
    errors: Vec<f64>,

    // Mouse
    mouse_info: String,

    // Object name
    object_name: String,

    // Pending file open (from drag-drop or dialog)
    pending_open: Option<String>,

    // Zoom / Pan
    zoom_level: f32,
    pan_offset: Vec2,
    fit_to_view: bool,

    // Drag-to-zoom selection (screen coords)
    drag_start: Option<Pos2>,
    drag_end: Option<Pos2>,
    is_panning: bool,
    ui_theme: UiTheme,
}

impl Default for FitsViewerApp {
    fn default() -> Self {
        Self {
            fits_doc: None,
            filepath: None,
            selected_hdu: 0,
            current_view: None,
            view_mode: ViewMode::Image,
            texture: None,
            needs_rerender: true,
            norm_index: 0,
            cmap_index: 0,
            vmin: 0.0,
            vmax: 1.0,
            data_min: 0.0,
            data_max: 1.0,
            vmin_slider: 0.0,
            vmax_slider: 1.0,
            is_cube: false,
            collapse_mode: false,
            integ_index: 0,
            frame_index: 0,
            frame_count: 0,
            wavelengths: Vec::new(),
            flux: Vec::new(),
            errors: Vec::new(),
            mouse_info: "READY".to_string(),
            object_name: "--".to_string(),
            pending_open: None,
            zoom_level: 1.0,
            pan_offset: Vec2::ZERO,
            fit_to_view: true,
            drag_start: None,
            drag_end: None,
            is_panning: false,
            ui_theme: UiTheme::Dark,
        }
    }
}

impl FitsViewerApp {
    /// Create a new app, optionally loading a file immediately.
    pub fn new(filepath: Option<String>) -> Self {
        let mut app = Self::default();
        if let Some(path) = filepath {
            app.open_file(&path);
        }
        app
    }

    /// Open and parse a FITS file.
    fn open_file(&mut self, raw_path: &str) {
        let path = crate::platform::decode_file_uri(raw_path);
        match FitsDocument::open(&path) {
            Ok(doc) => {
                // Find best initial HDU:
                // 1. Start with the first HDU that actually has renderable image data
                //    (handles files where PRIMARY has NAXIS=0 and the image is in HDU 1+)
                // 2. Then override with a spectrum HDU if one is detected
                let mut idx = 0;
                let mut found_image = false;
                for (i, hdu) in doc.hdus.iter().enumerate() {
                    if hdu.is_image && !hdu.shape.is_empty() && doc.images.contains_key(&i) {
                        if !found_image {
                            idx = i;
                            found_image = true;
                        }
                        // Prefer spectrum HDUs over plain images
                        if hdu.header_cards.contains_key("WAVEMIN")
                            || (hdu.shape.len() == 2 && hdu.shape[1] < 10)
                        {
                            idx = i;
                        }
                    }
                }
                self.filepath = Some(path);
                self.fits_doc = Some(doc);
                self.select_hdu(idx);
            }
            Err(e) => {
                self.mouse_info = format!("Error: {e}");
            }
        }
    }

    /// Reset zoom/pan to default.
    fn reset_zoom(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = Vec2::ZERO;
        self.drag_start = None;
        self.drag_end = None;
        self.is_panning = false;
    }

    /// Select an HDU by index and prepare the view.
    fn select_hdu(&mut self, index: usize) {
        self.selected_hdu = index;
        self.reset_zoom();
        let doc = match &self.fits_doc {
            Some(d) => d,
            None => return,
        };
        let hdu = match doc.hdus.get(index) {
            Some(h) => h.clone(),
            None => return,
        };

        self.object_name = hdu
            .header_cards
            .get("OBJECT")
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        if !hdu.is_image || hdu.shape.is_empty() {
            // Table view
            self.view_mode = ViewMode::Table;
            self.is_cube = false;
            self.current_view = None;
            self.texture = None;
            return;
        }

        // Check if it's a spectrum
        if spectrum::is_spectrum(&hdu) {
            self.view_mode = ViewMode::Spectrum;
            self.is_cube = false;
            self.prepare_spectrum(&hdu, index);
            return;
        }

        // Image or cube
        self.view_mode = ViewMode::Image;
        if let Some(img_data) = doc.images.get(&index) {
            match img_data {
                ImageData::Cube3D(cube) => {
                    self.is_cube = true;
                    self.frame_count = cube.shape()[0];
                    self.frame_index = 0;
                    self.collapse_mode = false;
                    self.update_image_view();
                }
                ImageData::Image2D(_) => {
                    self.is_cube = false;
                    self.update_image_view();
                }
                ImageData::Spectrum1D(data) => {
                    // 1D – treat as spectrum
                    self.view_mode = ViewMode::Spectrum;
                    self.wavelengths = spectrum::build_wavelength_axis(&hdu, data.len());
                    self.flux = data.clone();
                    self.errors.clear();
                }
            }
        }
    }

    /// Prepare spectrum data from HDU.
    fn prepare_spectrum(&mut self, hdu: &crate::fits_data::HduInfo, index: usize) {
        let doc = self.fits_doc.as_ref().unwrap();
        if let Some(img_data) = doc.images.get(&index) {
            match img_data {
                ImageData::Image2D(arr) => {
                    let nx = arr.ncols();
                    self.wavelengths = spectrum::build_wavelength_axis(hdu, nx);
                    self.flux = arr.row(0).to_vec();
                    if arr.nrows() > 1 {
                        self.errors = arr.row(1).to_vec();
                    } else {
                        self.errors.clear();
                    }
                }
                ImageData::Spectrum1D(data) => {
                    self.wavelengths = spectrum::build_wavelength_axis(hdu, data.len());
                    self.flux = data.clone();
                    self.errors.clear();
                }
                _ => {}
            }
        }
    }

    /// Update the 2D data view from the current image/cube + settings.
    fn update_image_view(&mut self) {
        let doc = match &self.fits_doc {
            Some(d) => d,
            None => return,
        };
        let img = match doc.images.get(&self.selected_hdu) {
            Some(i) => i,
            None => return,
        };

        let view: Array2<f64> = match img {
            ImageData::Image2D(arr) => arr.clone(),
            ImageData::Cube3D(cube) => {
                if self.collapse_mode {
                    match INTEG_METHODS[self.integ_index] {
                        "Sum" => cube.sum_axis(ndarray::Axis(0)),
                        "Mean" => cube
                            .mean_axis(ndarray::Axis(0))
                            .unwrap_or_else(|| cube.sum_axis(ndarray::Axis(0))),
                        "Max" => {
                            let shape = (cube.shape()[1], cube.shape()[2]);
                            let mut result = Array2::from_elem(shape, f64::NEG_INFINITY);
                            for frame in 0..cube.shape()[0] {
                                for y in 0..shape.0 {
                                    for x in 0..shape.1 {
                                        let v = cube[[frame, y, x]];
                                        if v > result[[y, x]] {
                                            result[[y, x]] = v;
                                        }
                                    }
                                }
                            }
                            result
                        }
                        _ => cube.sum_axis(ndarray::Axis(0)),
                    }
                } else {
                    let idx = self.frame_index.min(cube.shape()[0].saturating_sub(1));
                    cube.index_axis(ndarray::Axis(0), idx).to_owned()
                }
            }
            _ => return,
        };

        // Compute data range
        let mut dmin = f64::MAX;
        let mut dmax = f64::MIN;
        for &v in view.iter() {
            if v.is_finite() {
                if v < dmin {
                    dmin = v;
                }
                if v > dmax {
                    dmax = v;
                }
            }
        }
        self.data_min = dmin;
        self.data_max = dmax;

        // Compute initial display range based on algorithm
        let algo = algo_key(NORM_NAMES[self.norm_index]);
        if algo == "Linear" {
            let (z1, z2) = rendering::zscale_limits(&view);
            self.vmin = z1;
            self.vmax = z2;
        } else {
            self.vmin = dmin;
            self.vmax = dmax;
        }

        // Update slider positions
        let den = if (dmax - dmin).abs() > 1e-15 {
            dmax - dmin
        } else {
            1.0
        };
        self.vmin_slider = ((self.vmin - dmin) / den) as f32;
        self.vmax_slider = ((self.vmax - dmin) / den) as f32;

        self.current_view = Some(view);
        self.needs_rerender = true;
    }

    /// Rebuild the GPU texture from current_view + settings.
    fn rebuild_texture(&mut self, ctx: &egui::Context) {
        if let Some(data) = &self.current_view {
            let algo = algo_key(NORM_NAMES[self.norm_index]);
            let cmap = COLORMAP_NAMES[self.cmap_index];
            let (w, h, pixels) = rendering::render_to_rgba(data, self.vmin, self.vmax, algo, cmap);
            let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
            self.texture =
                Some(ctx.load_texture("fits_image", color_image, TextureOptions::NEAREST));
            self.needs_rerender = false;
        }
    }

    /// Show a native file-open dialog for FITS files.
    fn show_open_dialog(&mut self) {
        let result = rfd::FileDialog::new()
            .set_title("Open FITS File")
            .add_filter("FITS files", &["fits", "fit", "fts"])
            .add_filter("All files", &["*"])
            .pick_file();
        if let Some(path) = result {
            let path_str = path.to_string_lossy().to_string();
            self.open_file(&path_str);
        }
    }
}

// ── eframe::App implementation ───────────────────────────────────────────
impl eframe::App for FitsViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let palette = self.ui_theme.palette();

        // Apply selected theme
        let mut visuals = if self.ui_theme == UiTheme::Dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = palette.bg_main;
        visuals.window_fill = palette.bg_panel;
        visuals.widgets.noninteractive.bg_fill = palette.bg_panel;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text_primary);
        visuals.widgets.inactive.bg_fill = palette.widget_inactive_fill;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text_dim);
        visuals.widgets.hovered.bg_fill = palette.widget_hovered_fill;
        visuals.widgets.active.bg_fill = palette.widget_active_fill;
        visuals.selection.bg_fill = palette.selection_fill;
        visuals.selection.stroke = Stroke::new(1.0, palette.accent);
        ctx.set_visuals(visuals);

        // Handle Ctrl+O keyboard shortcut
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.show_open_dialog();
        }

        // Handle drag-and-drop
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    let path_str = path.to_string_lossy().to_string();
                    self.pending_open = Some(path_str);
                }
            }
        });
        if let Some(path) = self.pending_open.take() {
            self.open_file(&path);
        }

        // Rebuild texture if needed
        if self.needs_rerender && self.view_mode == ViewMode::Image {
            self.rebuild_texture(ctx);
        }

        // ── LEFT PANEL ───────────────────────────────────────────────
        egui::SidePanel::left("left_panel")
            .exact_width(280.0)
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::same(12),
                corner_radius: CornerRadius::same(12),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Theme").color(palette.text_primary).strong());
                    egui::ComboBox::from_id_salt("theme_combo")
                        .selected_text(match self.ui_theme {
                            UiTheme::Dark => "Dark",
                            UiTheme::RomaLight => "Light (Roma)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.ui_theme, UiTheme::Dark, "Dark");
                            ui.selectable_value(
                                &mut self.ui_theme,
                                UiTheme::RomaLight,
                                "Light (Roma)",
                            );
                        });
                });
                ui.add_space(8.0);

                // Open File button
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("📂 Open File")
                                .color(palette.accent)
                                .size(13.0),
                        )
                        .fill(palette.button_fill)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(Vec2::new(ui.available_width(), 32.0)),
                    )
                    .clicked()
                {
                    self.show_open_dialog();
                }
                ui.add_space(8.0);

                ui.label(
                    RichText::new("FILE STRUCTURE")
                        .color(palette.accent)
                        .strong()
                        .size(14.0),
                );
                ui.add_space(8.0);

                if let Some(doc) = &self.fits_doc {
                    let hdus = doc.hdus.clone();
                    ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        for hdu in &hdus {
                            let selected = self.selected_hdu == hdu.index;
                            let typ = if hdu.is_image { "IMG" } else { "TAB" };
                            let shape_str = if hdu.shape.is_empty() {
                                "(-)".to_string()
                            } else {
                                format!("{:?}", hdu.shape)
                            };
                            let label =
                                format!("{} | {}\n{} {}", hdu.index, hdu.name, typ, shape_str);

                            let bg = if selected {
                                Color32::from_rgba_premultiplied(
                                    palette.accent.r(),
                                    palette.accent.g(),
                                    palette.accent.b(),
                                    38,
                                )
                            } else {
                                Color32::TRANSPARENT
                            };
                            let text_color = if selected {
                                palette.accent
                            } else {
                                palette.text_dim
                            };

                            let resp = ui.add(
                                egui::Button::new(
                                    RichText::new(label).color(text_color).size(11.0),
                                )
                                .fill(bg)
                                .corner_radius(CornerRadius::same(6))
                                .stroke(if selected {
                                    Stroke::new(
                                        1.0,
                                        Color32::from_rgba_premultiplied(
                                            palette.accent.r(),
                                            palette.accent.g(),
                                            palette.accent.b(),
                                            77,
                                        ),
                                    )
                                } else {
                                    Stroke::NONE
                                })
                                .min_size(Vec2::new(ui.available_width(), 0.0)),
                            );
                            if resp.clicked() && self.selected_hdu != hdu.index {
                                self.select_hdu(hdu.index);
                            }
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("No file loaded")
                            .color(palette.text_dim)
                            .italics(),
                    );
                }

                ui.add_space(16.0);
                ui.label(
                    RichText::new("METADATA")
                        .color(palette.accent)
                        .strong()
                        .size(14.0),
                );
                ui.add_space(4.0);

                ScrollArea::vertical()
                    .id_salt("metadata_scroll")
                    .max_height(ui.available_height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(doc) = &self.fits_doc {
                            if let Some(hdu) = doc.hdus.get(self.selected_hdu) {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&hdu.header_text)
                                            .font(FontId::monospace(9.0))
                                            .color(palette.accent_secondary),
                                    )
                                    .selectable(true),
                                );
                            }
                        }
                    });
            });

        // ── RIGHT PANEL ──────────────────────────────────────────────
        egui::SidePanel::right("right_panel")
            .exact_width(260.0)
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::same(12),
                corner_radius: CornerRadius::same(12),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("APPEARANCE")
                        .color(palette.accent)
                        .strong()
                        .size(14.0),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Theme").color(palette.text_primary));
                    egui::ComboBox::from_id_salt("theme_combo_right")
                        .selected_text(match self.ui_theme {
                            UiTheme::Dark => "Dark",
                            UiTheme::RomaLight => "Light (Roma)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.ui_theme, UiTheme::Dark, "Dark");
                            ui.selectable_value(
                                &mut self.ui_theme,
                                UiTheme::RomaLight,
                                "Light (Roma)",
                            );
                        });
                });
                ui.add_space(12.0);

                if self.view_mode == ViewMode::Image {
                    // ── Visualization controls ───────────────────────
                    ui.label(
                        RichText::new("VISUALIZATION")
                            .color(palette.accent)
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(8.0);

                    ui.label(RichText::new("Algorithm:").color(palette.text_primary));
                    let prev_norm = self.norm_index;
                    egui::ComboBox::from_id_salt("norm_combo")
                        .selected_text(NORM_NAMES[self.norm_index])
                        .show_ui(ui, |ui| {
                            for (i, name) in NORM_NAMES.iter().enumerate() {
                                ui.selectable_value(&mut self.norm_index, i, *name);
                            }
                        });
                    if self.norm_index != prev_norm {
                        self.update_image_view();
                    }

                    ui.add_space(4.0);
                    ui.label(RichText::new("Colormap:").color(palette.text_primary));
                    let prev_cmap = self.cmap_index;
                    egui::ComboBox::from_id_salt("cmap_combo")
                        .selected_text(COLORMAP_NAMES[self.cmap_index])
                        .show_ui(ui, |ui| {
                            for (i, name) in COLORMAP_NAMES.iter().enumerate() {
                                ui.selectable_value(&mut self.cmap_index, i, *name);
                            }
                        });
                    if self.cmap_index != prev_cmap {
                        self.needs_rerender = true;
                    }

                    ui.add_space(12.0);

                    // VMIN
                    ui.label(RichText::new("VMIN").color(palette.text_primary));
                    let mut vmin_text = format!("{:.4}", self.vmin);
                    ui.add(
                        egui::TextEdit::singleline(&mut vmin_text)
                            .desired_width(80.0)
                            .text_color(palette.accent),
                    );
                    if let Ok(v) = vmin_text.parse::<f64>() {
                        if (v - self.vmin).abs() > 1e-8 {
                            self.vmin = v;
                            self.needs_rerender = true;
                        }
                    }
                    let prev_vmin_s = self.vmin_slider;
                    ui.add(egui::Slider::new(&mut self.vmin_slider, 0.0..=1.0).show_value(false));
                    if (self.vmin_slider - prev_vmin_s).abs() > 1e-5 {
                        self.vmin = self.data_min
                            + (self.vmin_slider as f64) * (self.data_max - self.data_min);
                        self.needs_rerender = true;
                    }

                    ui.add_space(4.0);

                    // VMAX
                    ui.label(RichText::new("VMAX").color(palette.text_primary));
                    let mut vmax_text = format!("{:.4}", self.vmax);
                    ui.add(
                        egui::TextEdit::singleline(&mut vmax_text)
                            .desired_width(80.0)
                            .text_color(palette.accent),
                    );
                    if let Ok(v) = vmax_text.parse::<f64>() {
                        if (v - self.vmax).abs() > 1e-8 {
                            self.vmax = v;
                            self.needs_rerender = true;
                        }
                    }
                    let prev_vmax_s = self.vmax_slider;
                    ui.add(egui::Slider::new(&mut self.vmax_slider, 0.0..=1.0).show_value(false));
                    if (self.vmax_slider - prev_vmax_s).abs() > 1e-5 {
                        self.vmax = self.data_min
                            + (self.vmax_slider as f64) * (self.data_max - self.data_min);
                        self.needs_rerender = true;
                    }

                    // ── Cube controls ────────────────────────────────
                    if self.is_cube {
                        ui.add_space(20.0);
                        ui.label(
                            RichText::new("CUBE")
                                .color(palette.accent)
                                .strong()
                                .size(14.0),
                        );
                        ui.add_space(4.0);

                        let prev_collapse = self.collapse_mode;
                        ui.checkbox(
                            &mut self.collapse_mode,
                            RichText::new("Collapse (2D)").color(palette.text_primary),
                        );

                        if self.collapse_mode {
                            let prev_integ = self.integ_index;
                            egui::ComboBox::from_id_salt("integ_combo")
                                .selected_text(INTEG_METHODS[self.integ_index])
                                .show_ui(ui, |ui| {
                                    for (i, name) in INTEG_METHODS.iter().enumerate() {
                                        ui.selectable_value(&mut self.integ_index, i, *name);
                                    }
                                });
                            if self.integ_index != prev_integ {
                                self.update_image_view();
                            }
                        }

                        if !self.collapse_mode {
                            ui.label(RichText::new("Frame:").color(palette.text_primary));
                            let prev_frame = self.frame_index;
                            let max_frame = self.frame_count.saturating_sub(1);
                            let mut fi = self.frame_index as i32;
                            ui.add(egui::Slider::new(&mut fi, 0..=(max_frame as i32)));
                            self.frame_index = fi as usize;
                            if self.frame_index != prev_frame {
                                self.update_image_view();
                            }
                        }

                        if self.collapse_mode != prev_collapse {
                            self.update_image_view();
                        }
                    }

                    // ── View controls ────────────────────────────────
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new("VIEW")
                            .color(palette.accent)
                            .strong()
                            .size(14.0),
                    );
                    ui.add_space(4.0);

                    ui.checkbox(
                        &mut self.fit_to_view,
                        RichText::new("Fit to View").color(palette.text_primary),
                    );

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Zoom: {:.0}%", self.zoom_level * 100.0))
                            .color(palette.text_dim),
                    );
                    ui.add_space(4.0);

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("⟲ Reset Zoom")
                                    .color(palette.accent)
                                    .size(12.0),
                            )
                            .fill(palette.button_fill)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(ui.available_width(), 28.0)),
                        )
                        .clicked()
                    {
                        self.reset_zoom();
                    }
                } else {
                    ui.label(
                        RichText::new("No image controls")
                            .color(palette.text_dim)
                            .italics(),
                    );
                }
            });

        // ── CENTER PANEL ─────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::same(0),
                corner_radius: CornerRadius::same(12),
                ..Default::default()
            })
            .show(ctx, |ui| {
                // Info bar
                ui.horizontal(|ui| {
                    ui.add_space(15.0);
                    ui.label(
                        RichText::new(format!("OBJECT: {}", self.object_name))
                            .color(palette.accent_secondary)
                            .strong()
                            .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(15.0);
                        ui.label(RichText::new(&self.mouse_info).color(palette.text_dim));
                    });
                });
                ui.add_space(4.0);

                match self.view_mode {
                    ViewMode::Image => {
                        if let Some(tex) = &self.texture {
                            let available = ui.available_size();
                            let tex_size = tex.size_vec2();

                            // Compute base scale: fit-to-view removes the .min(1.0) clamp
                            let base_scale = if self.fit_to_view {
                                (available.x / tex_size.x).min(available.y / tex_size.y)
                            } else {
                                (available.x / tex_size.x)
                                    .min(available.y / tex_size.y)
                                    .min(1.0)
                            };
                            let effective_scale = base_scale * self.zoom_level;
                            let display_size = tex_size * effective_scale;

                            // Allocate the full available area for interaction
                            let (resp, mut painter) =
                                ui.allocate_painter(available, egui::Sense::click_and_drag());
                            let canvas_rect = resp.rect;

                            // Image rect centered in canvas, offset by pan
                            let center = canvas_rect.center() + self.pan_offset;
                            let img_rect = Rect::from_center_size(center.into(), display_size);

                            // Clip and draw the image
                            painter.set_clip_rect(canvas_rect);
                            painter.image(
                                tex.id(),
                                img_rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );

                            // ── Scroll-wheel zoom (centered on cursor) ──
                            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                            if scroll_delta.abs() > 0.1 && resp.hovered() {
                                let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
                                let old_zoom = self.zoom_level;
                                self.zoom_level = (self.zoom_level * zoom_factor).clamp(0.1, 50.0);
                                // Zoom toward the cursor position
                                if let Some(cursor) = resp.hover_pos() {
                                    let cursor_vec = Vec2::new(cursor.x, cursor.y)
                                        - Vec2::new(center.x, center.y);
                                    let ratio = 1.0 - self.zoom_level / old_zoom;
                                    self.pan_offset += cursor_vec * ratio;
                                }
                                self.fit_to_view = false; // manual zoom overrides fit
                            }

                            // ── Double-click to reset ──
                            if resp.double_clicked() {
                                self.reset_zoom();
                            }

                            // ── Middle-click pan ──
                            if resp.dragged_by(egui::PointerButton::Middle) {
                                self.pan_offset += resp.drag_delta();
                                self.is_panning = true;
                            }
                            if resp.drag_stopped_by(egui::PointerButton::Middle) {
                                self.is_panning = false;
                            }

                            // ── Left-click drag-to-zoom ──
                            if resp.dragged_by(egui::PointerButton::Primary) && !self.is_panning {
                                if self.drag_start.is_none() {
                                    if let Some(pos) = resp.interact_pointer_pos() {
                                        self.drag_start = Some(pos);
                                    }
                                }
                                if let Some(pos) = resp.interact_pointer_pos() {
                                    self.drag_end = Some(pos);
                                }
                            }

                            // Draw selection rectangle
                            if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                                let sel_rect = Rect::from_two_pos(start, end);
                                if sel_rect.width() > 4.0 && sel_rect.height() > 4.0 {
                                    painter.rect_filled(
                                        sel_rect,
                                        0.0,
                                        Color32::from_rgba_premultiplied(
                                            palette.accent.r(),
                                            palette.accent.g(),
                                            palette.accent.b(),
                                            30,
                                        ),
                                    );
                                    painter.rect_stroke(
                                        sel_rect,
                                        0.0,
                                        Stroke::new(1.5, palette.accent),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }

                            // Commit drag-to-zoom on release
                            if resp.drag_stopped_by(egui::PointerButton::Primary)
                                && !self.is_panning
                            {
                                if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                                    let sel = Rect::from_two_pos(start, end);
                                    if sel.width() > 8.0 && sel.height() > 8.0 {
                                        // Compute zoom factor from selection
                                        let zoom_x = canvas_rect.width() / sel.width();
                                        let zoom_y = canvas_rect.height() / sel.height();
                                        let extra_zoom = zoom_x.min(zoom_y);

                                        // Pan so the selection center becomes the canvas center
                                        let sel_center = sel.center();
                                        let canvas_center = canvas_rect.center();
                                        let offset_before = Vec2::new(
                                            sel_center.x - canvas_center.x,
                                            sel_center.y - canvas_center.y,
                                        );

                                        self.pan_offset =
                                            (self.pan_offset - offset_before) * extra_zoom;
                                        self.zoom_level =
                                            (self.zoom_level * extra_zoom).clamp(0.1, 50.0);
                                        self.fit_to_view = false;
                                    }
                                }
                                self.drag_start = None;
                                self.drag_end = None;
                            }

                            // ── Mouse coordinate tracking ──
                            if let Some(pos) = resp.hover_pos() {
                                // Convert screen pos → image pixel
                                let frac_x = (pos.x - img_rect.left()) / img_rect.width();
                                let frac_y = (pos.y - img_rect.top()) / img_rect.height();
                                if frac_x >= 0.0 && frac_x <= 1.0 && frac_y >= 0.0 && frac_y <= 1.0
                                {
                                    let px = (frac_x * tex_size.x) as i32;
                                    let py = ((1.0 - frac_y) * tex_size.y) as i32;
                                    self.mouse_info = format!(
                                        "X: {}  Y: {}  | Zoom: {:.0}%",
                                        px,
                                        py,
                                        self.zoom_level * 100.0
                                    );
                                } else {
                                    self.mouse_info =
                                        format!("Zoom: {:.0}%", self.zoom_level * 100.0);
                                }
                            }
                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(ui.available_height() / 3.0);
                                if self.fits_doc.is_some() {
                                    // File is open but this HDU can't be rendered (e.g. compressed)
                                    ui.label(RichText::new("🗜").size(48.0));
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Compressed image — not yet supported")
                                            .color(palette.text_dim)
                                            .size(18.0),
                                    );
                                    ui.label(
                                        RichText::new("Header data is available in the left panel")
                                            .color(palette.text_dim)
                                            .size(13.0),
                                    );
                                } else {
                                    ui.label(RichText::new("🔭").size(48.0));
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Drop a FITS file here")
                                            .color(palette.text_dim)
                                            .size(18.0),
                                    );
                                    ui.label(
                                        RichText::new("or press Ctrl+O / click Open File")
                                            .color(palette.text_dim)
                                            .size(13.0),
                                    );
                                    ui.add_space(16.0);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("📂 Open File")
                                                    .color(palette.accent)
                                                    .size(15.0),
                                            )
                                            .fill(palette.button_fill)
                                            .corner_radius(CornerRadius::same(8))
                                            .min_size(Vec2::new(200.0, 40.0)),
                                        )
                                        .clicked()
                                    {
                                        self.show_open_dialog();
                                    }
                                }
                            });
                        }
                    }
                    ViewMode::Spectrum => {
                        self.draw_spectrum(ui);
                    }
                    ViewMode::Table => {
                        self.draw_table(ui);
                    }
                }
            });
    }
}

impl FitsViewerApp {
    /// Render spectrum plot using egui_plot.
    fn draw_spectrum(&self, ui: &mut egui::Ui) {
        let palette = self.ui_theme.palette();
        if self.wavelengths.is_empty() || self.flux.is_empty() {
            ui.label(RichText::new("No spectrum data").color(palette.text_dim));
            return;
        }

        let flux_points: PlotPoints = self
            .wavelengths
            .iter()
            .zip(self.flux.iter())
            .map(|(&w, &f)| [w, f])
            .collect();

        let flux_line = Line::new(flux_points)
            .color(palette.spectrum_line)
            .width(1.2)
            .name("Flux");

        Plot::new("spectrum_plot")
            .x_axis_label("Wavelength")
            .y_axis_label("Flux")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(flux_line);

                // Error band
                if !self.errors.is_empty() && self.errors.len() == self.flux.len() {
                    let mut poly_points: Vec<[f64; 2]> = Vec::new();
                    // Upper bound (forward)
                    for (i, (&w, &f)) in self.wavelengths.iter().zip(self.flux.iter()).enumerate() {
                        let err = self.errors[i];
                        poly_points.push([w, f + err]);
                    }
                    // Lower bound (reverse)
                    for (i, (&w, &f)) in self
                        .wavelengths
                        .iter()
                        .zip(self.flux.iter())
                        .enumerate()
                        .rev()
                    {
                        let err = self.errors[i];
                        poly_points.push([w, f - err]);
                    }
                    let error_band = Polygon::new(PlotPoints::from(poly_points))
                        .fill_color(Color32::from_rgba_premultiplied(
                            palette.spectrum_line.r(),
                            palette.spectrum_line.g(),
                            palette.spectrum_line.b(),
                            20,
                        ))
                        .stroke(Stroke::new(0.0, Color32::TRANSPARENT))
                        .name("Error");
                    plot_ui.polygon(error_band);
                }
            });
    }

    /// Render table data.
    fn draw_table(&self, ui: &mut egui::Ui) {
        let palette = self.ui_theme.palette();
        let doc = match &self.fits_doc {
            Some(d) => d,
            None => {
                ui.label(RichText::new("No data").color(palette.text_dim));
                return;
            }
        };
        let table = match doc.tables.get(&self.selected_hdu) {
            Some(t) => t,
            None => {
                ui.label(RichText::new("No table data for this HDU").color(palette.text_dim));
                return;
            }
        };

        ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("fits_table")
                .striped(true)
                .num_columns(table.columns.len())
                .show(ui, |ui| {
                    // Header
                    for col in &table.columns {
                        ui.label(RichText::new(col).color(palette.text_dim).strong());
                    }
                    ui.end_row();

                    // Rows
                    for row in &table.rows {
                        for val in row {
                            ui.label(RichText::new(val).color(palette.accent_secondary));
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
