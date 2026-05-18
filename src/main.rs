mod app;
mod fits_data;
mod platform;
mod rendering;
mod spectrum;

use app::FitsViewerApp;

fn main() -> eframe::Result<()> {
    env_logger::init();

    // Linux desktop integration (registers .desktop file)
    platform::ensure_linux_integration();

    // Parse CLI arguments (optional filepath)
    let filepath: Option<String> = std::env::args().nth(1).map(|arg| {
        platform::decode_file_uri(&arg)
    });

    // Window options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(
                filepath
                    .as_ref()
                    .map(|p| {
                        format!(
                            "AstroFITS - {}",
                            std::path::Path::new(p)
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        )
                    })
                    .unwrap_or_else(|| "AstroFITS Explorer".to_string()),
            )
            .with_inner_size([1400.0, 950.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AstroFITS Explorer",
        options,
        Box::new(move |_cc| Ok(Box::new(FitsViewerApp::new(filepath)))),
    )
}

use eframe::egui;
