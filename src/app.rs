use eframe::egui;
use egui::{
    Color32, CornerRadius, FontId, Frame, Margin, Pos2, Rect, RichText, ScrollArea, Stroke,
    TextureHandle, TextureOptions, Vec2,
};
use egui_plot::{Line, Plot, PlotPoints, Polygon};
use ndarray::Array2;
use std::collections::BTreeMap;

use crate::fits_data::{FitsDocument, HeaderCard, ImageData};
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

// Deep Space Obsidian Dark Theme
const DARK_PALETTE: ThemePalette = ThemePalette {
    bg_main: Color32::from_rgb(11, 14, 20),
    bg_panel: Color32::from_rgb(19, 24, 34),
    border: Color32::from_rgb(38, 49, 66),
    text_primary: Color32::from_rgb(241, 245, 249),
    text_dim: Color32::from_rgb(148, 163, 184),
    accent: Color32::from_rgb(56, 189, 248),
    accent_secondary: Color32::from_rgb(6, 182, 212),
    spectrum_line: Color32::from_rgb(16, 185, 129),
    button_fill: Color32::from_rgb(30, 41, 59),
    widget_inactive_fill: Color32::from_rgb(15, 23, 42),
    widget_hovered_fill: Color32::from_rgb(30, 41, 59),
    widget_active_fill: Color32::from_rgb(51, 65, 85),
    selection_fill: Color32::from_rgba_premultiplied(56, 189, 248, 45),
};

// Crisp Light Theme
const ROMA_LIGHT_PALETTE: ThemePalette = ThemePalette {
    bg_main: Color32::from_rgb(248, 250, 252),
    bg_panel: Color32::from_rgb(255, 255, 255),
    border: Color32::from_rgb(226, 232, 240),
    text_primary: Color32::from_rgb(15, 23, 42),
    text_dim: Color32::from_rgb(100, 116, 139),
    accent: Color32::from_rgb(153, 27, 27),
    accent_secondary: Color32::from_rgb(217, 119, 6),
    spectrum_line: Color32::from_rgb(153, 27, 27),
    button_fill: Color32::from_rgb(241, 245, 249),
    widget_inactive_fill: Color32::from_rgb(248, 250, 252),
    widget_hovered_fill: Color32::from_rgb(226, 232, 240),
    widget_active_fill: Color32::from_rgb(203, 213, 225),
    selection_fill: Color32::from_rgba_premultiplied(153, 27, 27, 35),
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

// Normalization algorithms
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

const INTEG_METHODS: [&str; 3] = ["Sum", "Mean", "Max"];

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Image,
    Spectrum,
    Table,
}

#[derive(Clone, Copy, PartialEq)]
enum SidebarTab {
    Display,
    HdusAndHeader,
    HeaderEditor,
}

pub struct FitsViewerApp {
    fits_doc: Option<FitsDocument>,
    filepath: Option<String>,
    selected_hdu: usize,
    current_view: Option<Array2<f64>>,
    view_mode: ViewMode,

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
    cube_animating: bool,

    // Spectrum data
    wavelengths: Vec<f64>,
    flux: Vec<f64>,
    errors: Vec<f64>,

    // Mouse & HUD
    mouse_info: String,
    hover_pixel_pos: Option<(i32, i32)>,
    hover_pixel_val: Option<f64>,

    object_name: String,
    pending_open: Option<String>,

    // Zoom / Pan
    zoom_level: f32,
    pan_offset: Vec2,
    fit_to_view: bool,

    drag_start: Option<Pos2>,
    drag_end: Option<Pos2>,
    is_panning: bool,
    ui_theme: UiTheme,

    // UI State
    sidebar_tab: SidebarTab,
    header_search: String,
    help_open: bool,

    // Header Editor
    draft_cards: BTreeMap<usize, Vec<HeaderCard>>,
    new_card_kw: String,
    new_card_val: String,
    new_card_comment: String,
    status_toast: Option<(String, bool)>,
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
            cube_animating: false,
            wavelengths: Vec::new(),
            flux: Vec::new(),
            errors: Vec::new(),
            mouse_info: "READY".to_string(),
            hover_pixel_pos: None,
            hover_pixel_val: None,
            object_name: "--".to_string(),
            pending_open: None,
            zoom_level: 1.0,
            pan_offset: Vec2::ZERO,
            fit_to_view: true,
            drag_start: None,
            drag_end: None,
            is_panning: false,
            ui_theme: UiTheme::Dark,
            sidebar_tab: SidebarTab::Display,
            header_search: String::new(),
            help_open: false,
            draft_cards: BTreeMap::new(),
            new_card_kw: String::new(),
            new_card_val: String::new(),
            new_card_comment: String::new(),
            status_toast: None,
        }
    }
}

impl FitsViewerApp {
    pub fn new(filepath: Option<String>) -> Self {
        let mut app = Self::default();
        if let Some(path) = filepath {
            app.open_file(&path);
        }
        app
    }

    fn open_file(&mut self, raw_path: &str) {
        let path = crate::platform::decode_file_uri(raw_path);
        match FitsDocument::open(&path) {
            Ok(doc) => {
                let mut idx = 0;
                let mut found_image = false;
                for (i, hdu) in doc.hdus.iter().enumerate() {
                    if hdu.is_image && !hdu.shape.is_empty() && doc.images.contains_key(&i) {
                        if !found_image {
                            idx = i;
                            found_image = true;
                        }
                        if hdu.header_cards.contains_key("WAVEMIN")
                            || (hdu.shape.len() == 2 && hdu.shape[1] < 10)
                        {
                            idx = i;
                        }
                    }
                }
                self.draft_cards.clear();
                for hdu in &doc.hdus {
                    self.draft_cards.insert(hdu.index, hdu.cards_list.clone());
                }
                self.status_toast = None;
                self.filepath = Some(path);
                self.fits_doc = Some(doc);
                self.select_hdu(idx);
            }
            Err(e) => {
                self.mouse_info = format!("Error: {e}");
            }
        }
    }

    fn reset_zoom(&mut self) {
        self.zoom_level = 1.0;
        self.pan_offset = Vec2::ZERO;
        self.drag_start = None;
        self.drag_end = None;
        self.is_panning = false;
        self.fit_to_view = true;
    }

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
            self.view_mode = ViewMode::Table;
            self.is_cube = false;
            self.current_view = None;
            self.texture = None;
            return;
        }

        if spectrum::is_spectrum(&hdu) {
            self.view_mode = ViewMode::Spectrum;
            self.is_cube = false;
            self.prepare_spectrum(&hdu, index);
            return;
        }

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
                    self.view_mode = ViewMode::Spectrum;
                    self.wavelengths = spectrum::build_wavelength_axis(&hdu, data.len());
                    self.flux = data.clone();
                    self.errors.clear();
                }
            }
        }
    }

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

        let mut dmin = f64::MAX;
        let mut dmax = f64::MIN;
        let mut has_finite = false;
        for &v in view.iter() {
            if v.is_finite() {
                has_finite = true;
                if v < dmin {
                    dmin = v;
                }
                if v > dmax {
                    dmax = v;
                }
            }
        }
        if !has_finite {
            dmin = 0.0;
            dmax = 1.0;
        } else if (dmax - dmin).abs() < 1e-15 {
            dmax = dmin + 1.0;
        }

        self.data_min = dmin;
        self.data_max = dmax;

        let algo = algo_key(NORM_NAMES[self.norm_index]);
        if algo == "Linear" {
            let (z1, z2) = rendering::zscale_limits(&view);
            self.vmin = z1;
            self.vmax = z2;
        } else {
            self.vmin = dmin;
            self.vmax = dmax;
        }

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

        // 3D Cube Animation playback
        if self.is_cube && self.cube_animating && !self.collapse_mode && self.frame_count > 1 {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
            self.frame_index = (self.frame_index + 1) % self.frame_count;
            self.update_image_view();
        }

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

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.show_open_dialog();
        }

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

        if self.needs_rerender && self.view_mode == ViewMode::Image {
            self.rebuild_texture(ctx);
        }

        // ═════════════════════════════════════════════════════════════
        // TOP CONTROL TOOLBAR
        // ═════════════════════════════════════════════════════════════
        egui::TopBottomPanel::top("top_toolbar")
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::symmetric(14, 8),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo & App Title
                    ui.label(
                        RichText::new("🔭 AstroFITS")
                            .strong()
                            .size(17.0)
                            .color(palette.accent),
                    );
                    ui.label(RichText::new("Explorer").size(14.0).color(palette.text_dim));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // File badge & HDU status
                    if let Some(path_str) = &self.filepath {
                        let filename = std::path::Path::new(path_str)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        ui.label(
                            RichText::new(format!("📁 {filename}"))
                                .color(palette.text_primary)
                                .strong(),
                        );
                        if let Some(doc) = &self.fits_doc {
                            if let Some(hdu) = doc.hdus.get(self.selected_hdu) {
                                ui.label(
                                    RichText::new(format!("[HDU {}: {}]", hdu.index, hdu.name))
                                        .color(palette.accent_secondary)
                                        .size(12.0),
                                );
                            }
                        }
                    } else {
                        ui.label(
                            RichText::new("No File Loaded")
                                .color(palette.text_dim)
                                .italics(),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Help Button
                        if ui
                            .button(RichText::new("❓ Help").color(palette.text_primary))
                            .clicked()
                        {
                            self.help_open = true;
                        }

                        // Theme switch button
                        let (theme_icon, theme_label) = match self.ui_theme {
                            UiTheme::Dark => ("🌙", "Dark"),
                            UiTheme::RomaLight => ("☀️", "Light"),
                        };
                        if ui
                            .button(RichText::new(format!("{theme_icon} {theme_label}")))
                            .clicked()
                        {
                            self.ui_theme = match self.ui_theme {
                                UiTheme::Dark => UiTheme::RomaLight,
                                UiTheme::RomaLight => UiTheme::Dark,
                            };
                        }

                        ui.separator();

                        // View reset
                        if ui
                            .button(RichText::new("⟲ Reset Zoom").color(palette.text_primary))
                            .clicked()
                        {
                            self.reset_zoom();
                        }

                        // Open file button
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("📂 Open File").color(palette.accent).strong(),
                                )
                                .fill(palette.button_fill)
                                .corner_radius(CornerRadius::same(6)),
                            )
                            .clicked()
                        {
                            self.show_open_dialog();
                        }
                    });
                });
            });

        // ═════════════════════════════════════════════════════════════
        // BOTTOM STATUS BAR
        // ═════════════════════════════════════════════════════════════
        egui::TopBottomPanel::bottom("status_bar")
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::symmetric(14, 4),
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("OBJECT: {}", self.object_name))
                            .color(palette.accent_secondary)
                            .strong()
                            .size(12.0),
                    );

                    if let Some(doc) = &self.fits_doc {
                        if let Some(hdu) = doc.hdus.get(self.selected_hdu) {
                            ui.separator();
                            let shape_str = if hdu.shape.is_empty() {
                                "Header Only".to_string()
                            } else {
                                format!("Dim: {:?}", hdu.shape)
                            };
                            ui.label(
                                RichText::new(shape_str)
                                    .color(palette.text_dim)
                                    .size(12.0),
                            );
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some((msg, is_success)) = &self.status_toast {
                            let color = if *is_success {
                                Color32::from_rgb(46, 160, 67)
                            } else {
                                Color32::from_rgb(248, 81, 73)
                            };
                            ui.label(RichText::new(msg).color(color).strong().size(12.0));
                        } else {
                            ui.label(
                                RichText::new(&self.mouse_info)
                                    .color(palette.text_dim)
                                    .size(12.0),
                            );
                        }
                    });
                });
            });

        // ═════════════════════════════════════════════════════════════
        // LEFT TABBED SIDEBAR
        // ═════════════════════════════════════════════════════════════
        egui::SidePanel::left("left_sidebar")
            .resizable(true)
            .default_width(320.0)
            .width_range(260.0..=650.0)
            .frame(Frame {
                fill: palette.bg_panel,
                stroke: Stroke::new(1.0, palette.border),
                inner_margin: Margin::same(10),
                corner_radius: CornerRadius::same(10),
                ..Default::default()
            })
            .show(ctx, |ui| {
                let avail_w = ui.available_width();
                let body_size = (avail_w / 24.0).clamp(9.5, 12.5);
                let mono_size = (avail_w / 28.0).clamp(8.0, 11.0);

                // Segmented tab selector
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.sidebar_tab,
                        SidebarTab::Display,
                        RichText::new("🎛 Controls").size(body_size),
                    );
                    ui.selectable_value(
                        &mut self.sidebar_tab,
                        SidebarTab::HdusAndHeader,
                        RichText::new("📂 HDUs").size(body_size),
                    );
                    ui.selectable_value(
                        &mut self.sidebar_tab,
                        SidebarTab::HeaderEditor,
                        RichText::new("✏️ Editor").size(body_size),
                    );
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                match self.sidebar_tab {
                    SidebarTab::Display => {
                        ScrollArea::vertical().show(ui, |ui| {
                            if self.view_mode == ViewMode::Image {
                                // Group 1: Stretch & Colormap
                                ui.group(|ui| {
                                    ui.label(
                                        RichText::new("STRETCH & COLOR")
                                            .color(palette.accent)
                                            .strong()
                                            .size(body_size),
                                    );
                                    ui.add_space(4.0);

                                    ui.label(RichText::new("Algorithm:").color(palette.text_primary));
                                    let prev_norm = self.norm_index;
                                    egui::ComboBox::from_id_salt("norm_combo")
                                        .selected_text(NORM_NAMES[self.norm_index])
                                        .width(avail_w * 0.85)
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
                                        .width(avail_w * 0.85)
                                        .show_ui(ui, |ui| {
                                            for (i, name) in COLORMAP_NAMES.iter().enumerate() {
                                                ui.selectable_value(&mut self.cmap_index, i, *name);
                                            }
                                        });
                                    if self.cmap_index != prev_cmap {
                                        self.needs_rerender = true;
                                    }
                                });

                                ui.add_space(8.0);

                                // Group 2: Intensity Range VMIN / VMAX
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("INTENSITY RANGE")
                                                .color(palette.accent)
                                                .strong()
                                                .size(body_size),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui
                                                    .button(RichText::new("⚡ Auto ZScale").size(body_size - 1.0))
                                                    .clicked()
                                                {
                                                    self.update_image_view();
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(4.0);

                                    // VMIN
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("VMIN").size(mono_size).color(palette.text_dim));
                                        let mut vmin_text = format!("{:.4}", self.vmin);
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(&mut vmin_text)
                                                    .desired_width(80.0),
                                            )
                                            .changed()
                                        {
                                            if let Ok(v) = vmin_text.parse::<f64>() {
                                                self.vmin = v;
                                                self.needs_rerender = true;
                                            }
                                        }
                                    });
                                    let prev_vmin_s = self.vmin_slider;
                                    ui.add(
                                        egui::Slider::new(&mut self.vmin_slider, 0.0..=1.0)
                                            .show_value(false),
                                    );
                                    if (self.vmin_slider - prev_vmin_s).abs() > 1e-5 {
                                        self.vmin = self.data_min
                                            + (self.vmin_slider as f64)
                                                * (self.data_max - self.data_min);
                                        self.needs_rerender = true;
                                    }

                                    ui.add_space(4.0);

                                    // VMAX
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("VMAX").size(mono_size).color(palette.text_dim));
                                        let mut vmax_text = format!("{:.4}", self.vmax);
                                        if ui
                                            .add(
                                                egui::TextEdit::singleline(&mut vmax_text)
                                                    .desired_width(80.0),
                                            )
                                            .changed()
                                        {
                                            if let Ok(v) = vmax_text.parse::<f64>() {
                                                self.vmax = v;
                                                self.needs_rerender = true;
                                            }
                                        }
                                    });
                                    let prev_vmax_s = self.vmax_slider;
                                    ui.add(
                                        egui::Slider::new(&mut self.vmax_slider, 0.0..=1.0)
                                            .show_value(false),
                                    );
                                    if (self.vmax_slider - prev_vmax_s).abs() > 1e-5 {
                                        self.vmax = self.data_min
                                            + (self.vmax_slider as f64)
                                                * (self.data_max - self.data_min);
                                        self.needs_rerender = true;
                                    }
                                });

                                // Group 3: 3D Cube Controls
                                if self.is_cube {
                                    ui.add_space(8.0);
                                    ui.group(|ui| {
                                        ui.label(
                                            RichText::new("🧊 3D CUBE CONTROLS")
                                                .color(palette.accent)
                                                .strong()
                                                .size(body_size),
                                        );
                                        ui.add_space(4.0);

                                        let prev_collapse = self.collapse_mode;
                                        ui.checkbox(
                                            &mut self.collapse_mode,
                                            RichText::new("Collapse to 2D Image").color(palette.text_primary),
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
                                        } else {
                                            ui.horizontal(|ui| {
                                                let play_label = if self.cube_animating { "⏸ Pause" } else { "▶ Play" };
                                                if ui.button(RichText::new(play_label).size(body_size)).clicked() {
                                                    self.cube_animating = !self.cube_animating;
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Frame {} / {}",
                                                        self.frame_index + 1,
                                                        self.frame_count
                                                    ))
                                                    .size(body_size),
                                                );
                                            });

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
                                    });
                                }
                            } else {
                                ui.label(
                                    RichText::new("No image controls for current view mode")
                                        .color(palette.text_dim)
                                        .italics(),
                                );
                            }
                        });
                    }
                    SidebarTab::HdusAndHeader => {
                        // HDU Selection List
                        ui.label(
                            RichText::new("FILE HDU STRUCTURE")
                                .color(palette.accent)
                                .strong()
                                .size(body_size),
                        );
                        ui.add_space(4.0);

                        if let Some(doc) = &self.fits_doc {
                            let hdus = doc.hdus.clone();
                            ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                                for hdu in &hdus {
                                    let selected = self.selected_hdu == hdu.index;
                                    let typ_icon = if hdu.is_image { "🖼" } else { "📋" };
                                    let shape_str = if hdu.shape.is_empty() {
                                        "(-)".to_string()
                                    } else {
                                        format!("{:?}", hdu.shape)
                                    };
                                    let label =
                                        format!("{} HDU {} | {}\n{}", typ_icon, hdu.index, hdu.name, shape_str);

                                    let bg = if selected {
                                        palette.selection_fill
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
                                            RichText::new(label).color(text_color).size(body_size - 1.0),
                                        )
                                        .fill(bg)
                                        .corner_radius(CornerRadius::same(6))
                                        .min_size(Vec2::new(avail_w, 0.0)),
                                    );
                                    if resp.clicked() && self.selected_hdu != hdu.index {
                                        self.select_hdu(hdu.index);
                                    }
                                }
                            });
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Header Inspector with Filter Box
                        ui.label(
                            RichText::new("HEADER KEYWORDS")
                                .color(palette.accent)
                                .strong()
                                .size(body_size),
                        );
                        ui.add_space(4.0);

                        ui.add(
                            egui::TextEdit::singleline(&mut self.header_search)
                                .hint_text("🔍 Filter keywords...")
                                .desired_width(avail_w),
                        );
                        ui.add_space(4.0);

                        ScrollArea::vertical()
                            .id_salt("hdr_inspect_scroll")
                            .max_height(ui.available_height())
                            .show(ui, |ui| {
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                let filter = self.header_search.trim().to_uppercase();
                                if let Some(cards) = self.draft_cards.get(&self.selected_hdu) {
                                    for card in cards {
                                        if card.keyword == "END" {
                                            continue;
                                        }
                                        if !filter.is_empty()
                                            && !card.keyword.contains(&filter)
                                            && !card.value.to_uppercase().contains(&filter)
                                            && !card.comment.to_uppercase().contains(&filter)
                                        {
                                            continue;
                                        }

                                        let line_str = if card.keyword == "COMMENT" || card.keyword == "HISTORY" || card.keyword.is_empty() {
                                            format!("{:<8} {}", card.keyword, card.comment)
                                        } else if !card.comment.is_empty() {
                                            format!("{:<8} = {:<16} / {}", card.keyword, card.value, card.comment)
                                        } else {
                                            format!("{:<8} = {:<16}", card.keyword, card.value)
                                        };

                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(line_str)
                                                    .font(FontId::monospace(mono_size))
                                                    .color(palette.accent_secondary),
                                            )
                                            .selectable(true),
                                        );
                                    }
                                }
                            });
                    }
                    SidebarTab::HeaderEditor => {
                        // Header Editor Mode Tab
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("💾 Save as _edited.fits")
                                        .color(Color32::WHITE)
                                        .strong()
                                        .size(body_size + 1.0),
                                )
                                .fill(palette.accent)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(avail_w, 32.0)),
                            )
                            .clicked()
                        {
                            if let Some(doc) = &self.fits_doc {
                                match doc.save_edited_to_new_file(&self.draft_cards) {
                                    Ok(new_path) => {
                                        self.status_toast = Some((
                                            format!("✅ Saved to:\n{}", new_path),
                                            true,
                                        ));
                                    }
                                    Err(e) => {
                                        self.status_toast = Some((format!("❌ Save error: {e}"), false));
                                    }
                                }
                            }
                        }
                        ui.add_space(8.0);

                        // Add new keyword card form
                        ui.group(|ui| {
                            ui.label(RichText::new("➕ Add New Keyword").strong().color(palette.accent).size(body_size));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_card_kw)
                                        .hint_text("KEY")
                                        .desired_width((avail_w * 0.35).max(60.0)),
                                );
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_card_val)
                                        .hint_text("VALUE")
                                        .desired_width((avail_w * 0.45).max(70.0)),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.new_card_comment)
                                        .hint_text("COMMENT")
                                        .desired_width((avail_w * 0.65).max(100.0)),
                                );
                                if ui.button(RichText::new("Add").size(body_size)).clicked()
                                    && !self.new_card_kw.trim().is_empty()
                                {
                                    let card = HeaderCard::new(
                                        &self.new_card_kw,
                                        &self.new_card_val,
                                        &self.new_card_comment,
                                    );
                                    let cards = self.draft_cards
                                        .entry(self.selected_hdu)
                                        .or_default();
                                    let insert_pos = cards.iter().position(|c| c.keyword == "END").unwrap_or(cards.len());
                                    cards.insert(insert_pos, card);
                                    self.new_card_kw.clear();
                                    self.new_card_val.clear();
                                    self.new_card_comment.clear();
                                }
                            });
                        });

                        ui.add_space(6.0);

                        ScrollArea::vertical()
                            .id_salt("editor_scroll_tab")
                            .max_height(ui.available_height())
                            .show(ui, |ui| {
                                if let Some(cards) = self.draft_cards.get_mut(&self.selected_hdu) {
                                    let mut delete_idx = None;
                                    for (idx, card) in cards.iter_mut().enumerate() {
                                        if card.keyword == "END" {
                                            continue;
                                        }
                                        ui.group(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(&card.keyword)
                                                        .strong()
                                                        .color(palette.accent)
                                                        .size(body_size),
                                                );
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(egui::Align::Center),
                                                    |ui| {
                                                        if ui.small_button("🗑").clicked() {
                                                            delete_idx = Some(idx);
                                                        }
                                                    },
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("Val:")
                                                        .size(mono_size)
                                                        .color(palette.text_dim),
                                                );
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut card.value)
                                                        .desired_width(avail_w * 0.65),
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("Com:")
                                                        .size(mono_size)
                                                        .color(palette.text_dim),
                                                );
                                                ui.add(
                                                    egui::TextEdit::singleline(&mut card.comment)
                                                        .desired_width(avail_w * 0.65),
                                                );
                                            });
                                        });
                                        ui.add_space(2.0);
                                    }
                                    if let Some(idx) = delete_idx {
                                        cards.remove(idx);
                                    }
                                }
                            });
                    }
                }
            });

        // ═════════════════════════════════════════════════════════════
        // MAIN CANVAS (CENTRAL PANEL)
        // ═════════════════════════════════════════════════════════════
        egui::CentralPanel::default()
            .frame(Frame {
                fill: palette.bg_main,
                stroke: Stroke::NONE,
                inner_margin: Margin::same(0),
                ..Default::default()
            })
            .show(ctx, |ui| {
                match self.view_mode {
                    ViewMode::Image => {
                        if let Some(tex) = &self.texture {
                            let available = ui.available_size();
                            let tex_size = tex.size_vec2();

                            let base_scale = if self.fit_to_view {
                                (available.x / tex_size.x).min(available.y / tex_size.y)
                            } else {
                                (available.x / tex_size.x)
                                    .min(available.y / tex_size.y)
                                    .min(1.0)
                            };
                            let effective_scale = base_scale * self.zoom_level;
                            let display_size = tex_size * effective_scale;

                            let (resp, mut painter) =
                                ui.allocate_painter(available, egui::Sense::click_and_drag());
                            let canvas_rect = resp.rect;

                            let center = canvas_rect.center() + self.pan_offset;
                            let img_rect = Rect::from_center_size(center.into(), display_size);

                            painter.set_clip_rect(canvas_rect);
                            painter.image(
                                tex.id(),
                                img_rect,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );

                            // Scroll-wheel zoom
                            let scroll_delta = ctx.input(|i| i.smooth_scroll_delta.y);
                            if scroll_delta.abs() > 0.1 && resp.hovered() {
                                let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 1.0 / 1.1 };
                                let old_zoom = self.zoom_level;
                                self.zoom_level = (self.zoom_level * zoom_factor).clamp(0.1, 50.0);
                                if let Some(cursor) = resp.hover_pos() {
                                    let cursor_vec = Vec2::new(cursor.x, cursor.y)
                                        - Vec2::new(center.x, center.y);
                                    let ratio = 1.0 - self.zoom_level / old_zoom;
                                    self.pan_offset += cursor_vec * ratio;
                                }
                                self.fit_to_view = false;
                            }

                            if resp.double_clicked() {
                                self.reset_zoom();
                            }

                            if resp.dragged_by(egui::PointerButton::Middle) {
                                self.pan_offset += resp.drag_delta();
                                self.is_panning = true;
                            }
                            if resp.drag_stopped_by(egui::PointerButton::Middle) {
                                self.is_panning = false;
                            }

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

                            // Selection rectangle
                            if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                                let sel_rect = Rect::from_two_pos(start, end);
                                if sel_rect.width() > 4.0 && sel_rect.height() > 4.0 {
                                    painter.rect_filled(
                                        sel_rect,
                                        0.0,
                                        palette.selection_fill,
                                    );
                                    painter.rect_stroke(
                                        sel_rect,
                                        0.0,
                                        Stroke::new(1.5, palette.accent),
                                        egui::StrokeKind::Outside,
                                    );
                                }
                            }

                            if resp.drag_stopped_by(egui::PointerButton::Primary)
                                && !self.is_panning
                            {
                                if let (Some(start), Some(end)) = (self.drag_start, self.drag_end) {
                                    let sel = Rect::from_two_pos(start, end);
                                    if sel.width() > 8.0 && sel.height() > 8.0 {
                                        let zoom_x = canvas_rect.width() / sel.width();
                                        let zoom_y = canvas_rect.height() / sel.height();
                                        let extra_zoom = zoom_x.min(zoom_y);

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

                            // Hover coordinate tracking & pixel sampling
                            if let Some(pos) = resp.hover_pos() {
                                let frac_x = (pos.x - img_rect.left()) / img_rect.width();
                                let frac_y = (pos.y - img_rect.top()) / img_rect.height();
                                if frac_x >= 0.0 && frac_x <= 1.0 && frac_y >= 0.0 && frac_y <= 1.0
                                {
                                    let px = (frac_x * tex_size.x) as i32;
                                    let py = ((1.0 - frac_y) * tex_size.y) as i32;
                                    self.hover_pixel_pos = Some((px, py));

                                    // Extract data pixel value
                                    if let Some(data) = &self.current_view {
                                        let ry = (data.nrows() as i32 - 1 - py).max(0) as usize;
                                        let rx = px.max(0) as usize;
                                        if ry < data.nrows() && rx < data.ncols() {
                                            let val = data[[ry, rx]];
                                            self.hover_pixel_val = if val.is_nan() { None } else { Some(val) };
                                        }
                                    }
                                    self.mouse_info = format!("X: {px}  Y: {py} | Zoom: {:.0}%", self.zoom_level * 100.0);
                                } else {
                                    self.hover_pixel_pos = None;
                                    self.hover_pixel_val = None;
                                    self.mouse_info = format!("Zoom: {:.0}%", self.zoom_level * 100.0);
                                }
                            }

                            // ── FLOATING CANVAS HUD OVERLAY (TOP-RIGHT) ──
                            let hud_rect = Rect::from_min_size(
                                Pos2::new(canvas_rect.right() - 210.0, canvas_rect.top() + 14.0),
                                Vec2::new(196.0, 110.0),
                            );
                            painter.rect_filled(
                                hud_rect,
                                8.0,
                                Color32::from_black_alpha(200),
                            );
                            painter.rect_stroke(
                                hud_rect,
                                8.0,
                                Stroke::new(1.0, palette.border),
                                egui::StrokeKind::Outside,
                            );

                            let mut hud_ui = ui.new_child(egui::UiBuilder::new().max_rect(hud_rect.shrink(10.0)));
                            hud_ui.vertical(|ui| {
                                ui.label(RichText::new("CANVAS HUD").strong().size(11.0).color(palette.accent));
                                if let Some((px, py)) = self.hover_pixel_pos {
                                    ui.label(RichText::new(format!("X: {px}   Y: {py}")).font(FontId::monospace(11.0)).color(palette.text_primary));
                                } else {
                                    ui.label(RichText::new("Cursor: Outside").size(11.0).color(palette.text_dim));
                                }
                                if let Some(val) = self.hover_pixel_val {
                                    ui.label(RichText::new(format!("Pixel Value: {:.4}", val)).font(FontId::monospace(11.0)).color(palette.accent_secondary));
                                } else {
                                    ui.label(RichText::new("Pixel Value: --").size(11.0).color(palette.text_dim));
                                }
                                ui.label(RichText::new(format!("Zoom: {:.0}%", self.zoom_level * 100.0)).size(11.0).color(palette.text_dim));
                            });

                            // ── FLOATING QUICK ZOOM TOOLBAR (TOP-LEFT) ──
                            let zoom_tb_rect = Rect::from_min_size(
                                Pos2::new(canvas_rect.left() + 14.0, canvas_rect.top() + 14.0),
                                Vec2::new(140.0, 36.0),
                            );
                            painter.rect_filled(
                                zoom_tb_rect,
                                6.0,
                                Color32::from_black_alpha(200),
                            );
                            painter.rect_stroke(
                                zoom_tb_rect,
                                6.0,
                                Stroke::new(1.0, palette.border),
                                egui::StrokeKind::Outside,
                            );

                            let mut zb_ui = ui.new_child(egui::UiBuilder::new().max_rect(zoom_tb_rect.shrink(4.0)));
                            zb_ui.horizontal(|ui| {
                                if ui.button(RichText::new("➕").size(12.0)).clicked() {
                                    self.zoom_level = (self.zoom_level * 1.2).clamp(0.1, 50.0);
                                    self.fit_to_view = false;
                                }
                                if ui.button(RichText::new("➖").size(12.0)).clicked() {
                                    self.zoom_level = (self.zoom_level / 1.2).clamp(0.1, 50.0);
                                    self.fit_to_view = false;
                                }
                                if ui.button(RichText::new("⟲").size(12.0)).clicked() {
                                    self.reset_zoom();
                                }
                                if ui.button(RichText::new("🔲").size(12.0)).clicked() {
                                    self.fit_to_view = true;
                                    self.zoom_level = 1.0;
                                    self.pan_offset = Vec2::ZERO;
                                }
                            });

                        } else {
                            ui.vertical_centered(|ui| {
                                ui.add_space(ui.available_height() / 3.0);
                                if self.fits_doc.is_some() {
                                    ui.label(RichText::new("🗜").size(48.0));
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Compressed or non-renderable HDU")
                                            .color(palette.text_dim)
                                            .size(18.0),
                                    );
                                    ui.label(
                                        RichText::new("Header keywords are available in the left panel")
                                            .color(palette.text_dim)
                                            .size(13.0),
                                    );
                                } else {
                                    ui.label(RichText::new("🔭").size(54.0));
                                    ui.add_space(12.0);
                                    ui.label(
                                        RichText::new("Drop a FITS file here to view")
                                            .color(palette.accent)
                                            .strong()
                                            .size(20.0),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new("or press Ctrl+O / click Open File")
                                            .color(palette.text_dim)
                                            .size(14.0),
                                    );
                                    ui.add_space(20.0);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("📂 Open FITS File")
                                                    .color(Color32::WHITE)
                                                    .strong()
                                                    .size(15.0),
                                            )
                                            .fill(palette.accent)
                                            .corner_radius(CornerRadius::same(8))
                                            .min_size(Vec2::new(220.0, 44.0)),
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

        // ═════════════════════════════════════════════════════════════
        // HELP MODAL DIALOG
        // ═════════════════════════════════════════════════════════════
        if self.help_open {
            egui::Window::new("❓ AstroFITS Explorer Manual & Shortcuts")
                .collapsible(false)
                .resizable(false)
                .open(&mut self.help_open)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.set_max_width(450.0);
                    ui.heading("Keyboard & Mouse Controls");
                    ui.add_space(8.0);
                    egui::Grid::new("help_grid").striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Ctrl + O").strong());
                        ui.label("Open FITS file dialog");
                        ui.end_row();

                        ui.label(RichText::new("Mouse Scroll").strong());
                        ui.label("Zoom in / out centered at cursor");
                        ui.end_row();

                        ui.label(RichText::new("Middle Drag").strong());
                        ui.label("Pan image around canvas");
                        ui.end_row();

                        ui.label(RichText::new("Left Drag").strong());
                        ui.label("Drag selection box to zoom region");
                        ui.end_row();

                        ui.label(RichText::new("Double Click").strong());
                        ui.label("Reset zoom and centering");
                        ui.end_row();
                    });
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(RichText::new("Header Editor:").strong());
                    ui.label("Edit keywords in the left sidebar Editor tab. Saves safely to filename_edited.fits without touching the original file.");
                });
        }
    }
}

impl FitsViewerApp {
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
            .width(1.5)
            .name("Flux");

        Plot::new("spectrum_plot")
            .x_axis_label("Wavelength")
            .y_axis_label("Flux")
            .legend(egui_plot::Legend::default())
            .show(ui, |plot_ui| {
                plot_ui.line(flux_line);

                if !self.errors.is_empty() && self.errors.len() == self.flux.len() {
                    let mut poly_points: Vec<[f64; 2]> = Vec::new();
                    for (i, (&w, &f)) in self.wavelengths.iter().zip(self.flux.iter()).enumerate() {
                        let err = self.errors[i];
                        poly_points.push([w, f + err]);
                    }
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
                            25,
                        ))
                        .stroke(Stroke::new(0.0, Color32::TRANSPARENT))
                        .name("Error");
                    plot_ui.polygon(error_band);
                }
            });
    }

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
                    for col in &table.columns {
                        ui.label(RichText::new(col).color(palette.accent).strong());
                    }
                    ui.end_row();

                    for row in &table.rows {
                        for val in row {
                            ui.label(RichText::new(val).color(palette.text_primary));
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
