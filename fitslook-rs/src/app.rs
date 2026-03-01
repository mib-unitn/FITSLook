use eframe::egui;
use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, RichText, ScrollArea, Stroke, TextureHandle,
    TextureOptions, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints, Polygon};
use ndarray::Array2;

use crate::fits_data::{FitsDocument, ImageData};
use crate::rendering::{self, COLORMAP_NAMES};
use crate::spectrum;

// ── Color palette (matches the Python "Liquid Glass" dark theme) ─────────
const BG_MAIN: Color32 = Color32::from_rgb(13, 17, 23);
const BG_PANEL: Color32 = Color32::from_rgb(22, 27, 34);
const BORDER: Color32 = Color32::from_rgb(48, 54, 61);
const TEXT_PRIMARY: Color32 = Color32::from_rgb(201, 209, 217);
const TEXT_DIM: Color32 = Color32::from_rgb(139, 148, 158);
const ACCENT: Color32 = Color32::from_rgb(88, 166, 255);
const ACCENT_BLUE: Color32 = Color32::from_rgb(121, 192, 255);
const SPECTRUM_CYAN: Color32 = Color32::from_rgb(0, 240, 255);

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
                // Find best initial HDU (prefer spectrum or first image with data)
                let mut idx = 0;
                for (i, hdu) in doc.hdus.iter().enumerate() {
                    if hdu.is_image {
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

    /// Select an HDU by index and prepare the view.
    fn select_hdu(&mut self, index: usize) {
        self.selected_hdu = index;
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
                        "Mean" => cube.mean_axis(ndarray::Axis(0)).unwrap_or_else(|| cube.sum_axis(ndarray::Axis(0))),
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
                if v < dmin { dmin = v; }
                if v > dmax { dmax = v; }
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
        let den = if (dmax - dmin).abs() > 1e-15 { dmax - dmin } else { 1.0 };
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
            self.texture = Some(ctx.load_texture(
                "fits_image",
                color_image,
                TextureOptions::NEAREST,
            ));
            self.needs_rerender = false;
        }
    }
}

// ── eframe::App implementation ───────────────────────────────────────────
impl eframe::App for FitsViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_MAIN;
        visuals.window_fill = BG_PANEL;
        visuals.widgets.noninteractive.bg_fill = BG_PANEL;
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(13, 17, 23);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(30, 37, 48);
        visuals.widgets.active.bg_fill = Color32::from_rgb(40, 50, 65);
        visuals.selection.bg_fill = Color32::from_rgba_premultiplied(88, 166, 255, 40);
        visuals.selection.stroke = Stroke::new(1.0, ACCENT);
        ctx.set_visuals(visuals);

        // Rebuild texture if needed
        if self.needs_rerender && self.view_mode == ViewMode::Image {
            self.rebuild_texture(ctx);
        }

        // ── LEFT PANEL ───────────────────────────────────────────────
        egui::SidePanel::left("left_panel")
            .exact_width(280.0)
            .frame(Frame {
                fill: BG_PANEL,
                stroke: Stroke::new(1.0, BORDER),
                inner_margin: Margin::same(12),
                corner_radius: CornerRadius::same(12),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.label(RichText::new("FILE STRUCTURE").color(ACCENT).strong().size(14.0));
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
                            let label = format!("{} | {}\n{} {}", hdu.index, hdu.name, typ, shape_str);

                            let bg = if selected {
                                Color32::from_rgba_premultiplied(88, 166, 255, 38)
                            } else {
                                Color32::TRANSPARENT
                            };
                            let text_color = if selected { ACCENT } else { TEXT_DIM };

                            let resp = ui.add(
                                egui::Button::new(RichText::new(label).color(text_color).size(11.0))
                                    .fill(bg)
                                    .corner_radius(CornerRadius::same(6))
                                    .stroke(if selected {
                                        Stroke::new(1.0, Color32::from_rgba_premultiplied(88, 166, 255, 77))
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
                    ui.label(RichText::new("No file loaded").color(TEXT_DIM).italics());
                }

                ui.add_space(16.0);
                ui.label(RichText::new("METADATA").color(ACCENT).strong().size(14.0));
                ui.add_space(4.0);

                ScrollArea::vertical().show(ui, |ui| {
                    if let Some(doc) = &self.fits_doc {
                        if let Some(hdu) = doc.hdus.get(self.selected_hdu) {
                            ui.add(
                                egui::TextEdit::multiline(&mut hdu.header_text.as_str())
                                    .font(FontId::monospace(9.0))
                                    .text_color(ACCENT_BLUE)
                                    .desired_width(f32::INFINITY),
                            );
                        }
                    }
                });
            });

        // ── RIGHT PANEL ──────────────────────────────────────────────
        egui::SidePanel::right("right_panel")
            .exact_width(260.0)
            .frame(Frame {
                fill: BG_PANEL,
                stroke: Stroke::new(1.0, BORDER),
                inner_margin: Margin::same(12),
                corner_radius: CornerRadius::same(12),
                ..Default::default()
            })
            .show(ctx, |ui| {
                if self.view_mode == ViewMode::Image {
                    // ── Visualization controls ───────────────────────
                    ui.label(RichText::new("VISUALIZATION").color(ACCENT).strong().size(14.0));
                    ui.add_space(8.0);

                    ui.label(RichText::new("Algorithm:").color(TEXT_PRIMARY));
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
                    ui.label(RichText::new("Colormap:").color(TEXT_PRIMARY));
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
                    ui.label(RichText::new("VMIN").color(TEXT_PRIMARY));
                    let mut vmin_text = format!("{:.4}", self.vmin);
                    ui.add(egui::TextEdit::singleline(&mut vmin_text)
                        .desired_width(80.0)
                        .text_color(ACCENT));
                    if let Ok(v) = vmin_text.parse::<f64>() {
                        if (v - self.vmin).abs() > 1e-8 {
                            self.vmin = v;
                            self.needs_rerender = true;
                        }
                    }
                    let prev_vmin_s = self.vmin_slider;
                    ui.add(egui::Slider::new(&mut self.vmin_slider, 0.0..=1.0).show_value(false));
                    if (self.vmin_slider - prev_vmin_s).abs() > 1e-5 {
                        self.vmin = self.data_min + (self.vmin_slider as f64) * (self.data_max - self.data_min);
                        self.needs_rerender = true;
                    }

                    ui.add_space(4.0);

                    // VMAX
                    ui.label(RichText::new("VMAX").color(TEXT_PRIMARY));
                    let mut vmax_text = format!("{:.4}", self.vmax);
                    ui.add(egui::TextEdit::singleline(&mut vmax_text)
                        .desired_width(80.0)
                        .text_color(ACCENT));
                    if let Ok(v) = vmax_text.parse::<f64>() {
                        if (v - self.vmax).abs() > 1e-8 {
                            self.vmax = v;
                            self.needs_rerender = true;
                        }
                    }
                    let prev_vmax_s = self.vmax_slider;
                    ui.add(egui::Slider::new(&mut self.vmax_slider, 0.0..=1.0).show_value(false));
                    if (self.vmax_slider - prev_vmax_s).abs() > 1e-5 {
                        self.vmax = self.data_min + (self.vmax_slider as f64) * (self.data_max - self.data_min);
                        self.needs_rerender = true;
                    }

                    // ── Cube controls ────────────────────────────────
                    if self.is_cube {
                        ui.add_space(20.0);
                        ui.label(RichText::new("CUBE").color(ACCENT).strong().size(14.0));
                        ui.add_space(4.0);

                        let prev_collapse = self.collapse_mode;
                        ui.checkbox(&mut self.collapse_mode, RichText::new("Collapse (2D)").color(TEXT_PRIMARY));

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
                            ui.label(RichText::new("Frame:").color(TEXT_PRIMARY));
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
                } else {
                    ui.label(RichText::new("No image controls").color(TEXT_DIM).italics());
                }
            });

        // ── CENTER PANEL ─────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame {
                fill: BG_PANEL,
                stroke: Stroke::new(1.0, BORDER),
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
                            .color(ACCENT_BLUE)
                            .strong()
                            .size(14.0),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(15.0);
                        ui.label(RichText::new(&self.mouse_info).color(TEXT_DIM));
                    });
                });
                ui.add_space(4.0);

                match self.view_mode {
                    ViewMode::Image => {
                        if let Some(tex) = &self.texture {
                            let available = ui.available_size();
                            let tex_size = tex.size_vec2();
                            let scale = (available.x / tex_size.x).min(available.y / tex_size.y).min(1.0);
                            let display_size = tex_size * scale;

                            let resp = ui.add(
                                egui::Image::new(tex)
                                    .fit_to_exact_size(display_size)
                                    .sense(egui::Sense::hover()),
                            );

                            if let Some(pos) = resp.hover_pos() {
                                let rect = resp.rect;
                                let frac_x = (pos.x - rect.left()) / rect.width();
                                let frac_y = (pos.y - rect.top()) / rect.height();
                                let px = (frac_x * tex_size.x) as i32;
                                let py = ((1.0 - frac_y) * tex_size.y) as i32; // flip Y
                                self.mouse_info = format!("X: {}  Y: {}", px, py);
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    RichText::new("Open a FITS file to view")
                                        .color(TEXT_DIM)
                                        .size(18.0),
                                );
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
        if self.wavelengths.is_empty() || self.flux.is_empty() {
            ui.label(RichText::new("No spectrum data").color(TEXT_DIM));
            return;
        }

        let flux_points: PlotPoints = self
            .wavelengths
            .iter()
            .zip(self.flux.iter())
            .map(|(&w, &f)| [w, f])
            .collect();

        let flux_line = Line::new(flux_points)
            .color(SPECTRUM_CYAN)
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
                    for (i, (&w, &f)) in self.wavelengths.iter().zip(self.flux.iter()).enumerate().rev() {
                        let err = self.errors[i];
                        poly_points.push([w, f - err]);
                    }
                    let error_band = Polygon::new(PlotPoints::from(poly_points))
                        .fill_color(Color32::from_rgba_premultiplied(0, 240, 255, 20))
                        .stroke(Stroke::new(0.0, Color32::TRANSPARENT))
                        .name("Error");
                    plot_ui.polygon(error_band);
                }
            });
    }

    /// Render table data.
    fn draw_table(&self, ui: &mut egui::Ui) {
        let doc = match &self.fits_doc {
            Some(d) => d,
            None => {
                ui.label(RichText::new("No data").color(TEXT_DIM));
                return;
            }
        };
        let table = match doc.tables.get(&self.selected_hdu) {
            Some(t) => t,
            None => {
                ui.label(RichText::new("No table data for this HDU").color(TEXT_DIM));
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
                        ui.label(RichText::new(col).color(TEXT_DIM).strong());
                    }
                    ui.end_row();

                    // Rows
                    for row in &table.rows {
                        for val in row {
                            ui.label(RichText::new(val).color(ACCENT_BLUE));
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
