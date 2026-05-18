use crate::fits_data::HduInfo;

/// Detect whether an HDU contains spectrum data rather than a 2D image.
///
/// Mirrors the Python logic:
///   - header has WAVEMIN, OR
///   - data is 1D, OR
///   - data is 2D with first axis < 10 (flux + error rows)
pub fn is_spectrum(hdu: &HduInfo) -> bool {
    if hdu.header_cards.contains_key("WAVEMIN") {
        return true;
    }
    let shape = &hdu.shape;
    if shape.len() == 1 {
        return true;
    }
    // 2D with small first axis (the NY dimension in FITS, stored as shape[1] after our reorder)
    // In our parser, shape = [NAXIS1, NAXIS2, ...], so NAXIS2 < 10 means few rows
    if shape.len() == 2 && shape[1] < 10 {
        return true;
    }
    false
}

/// Build a wavelength array from header values.
/// Returns a vector of wavelength sample points.
pub fn build_wavelength_axis(hdu: &HduInfo, n_pixels: usize) -> Vec<f64> {
    let wmin: f64 = hdu
        .header_cards
        .get("WAVEMIN")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let wmax: f64 = hdu
        .header_cards
        .get("WAVEMAX")
        .and_then(|v| v.parse().ok())
        .unwrap_or(n_pixels as f64);

    (0..n_pixels)
        .map(|i| {
            if n_pixels > 1 {
                wmin + (wmax - wmin) * (i as f64) / ((n_pixels - 1) as f64)
            } else {
                wmin
            }
        })
        .collect()
}
