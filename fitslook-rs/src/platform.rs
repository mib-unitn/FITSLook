use std::fs;
use std::path::Path;
use std::process::Command;

/// Decode a `file://` URI to a regular file path.
/// Handles percent-encoded characters (e.g. `%20` → space).
pub fn decode_file_uri(uri: &str) -> String {
    if uri.starts_with("file://") {
        // Parse as URL and extract the path (handles percent-decoding)
        if let Ok(parsed) = url::Url::parse(uri) {
            return parsed.path().to_string();
        }
        // Fallback: strip prefix
        uri.strip_prefix("file://").unwrap_or(uri).to_string()
    } else {
        uri.to_string()
    }
}

/// Create a `.desktop` file so Linux file managers can "Open With" FITSLook.
pub fn ensure_linux_integration() {
    if !cfg!(target_os = "linux") {
        return;
    }

    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    if exe.is_empty() {
        return;
    }

    let desktop_dir = dirs_path("applications");
    if let Some(dir) = &desktop_dir {
        fs::create_dir_all(dir).ok();
        let desktop_path = Path::new(dir).join("fitslook.desktop");
        let content = format!(
            "[Desktop Entry]\n\
             Name=AstroFITS\n\
             Exec=\"{exe}\" %u\n\
             Type=Application\n\
             MimeType=image/fits;image/x-fits;application/fits;\n"
        );
        fs::write(&desktop_path, content).ok();
        Command::new("update-desktop-database")
            .arg(dir)
            .output()
            .ok();
    }
}

/// Get the XDG data path for a subdirectory, e.g. `~/.local/share/applications`.
fn dirs_path(subdir: &str) -> Option<String> {
    std::env::var("HOME").ok().map(|home| {
        format!("{home}/.local/share/{subdir}")
    })
}
