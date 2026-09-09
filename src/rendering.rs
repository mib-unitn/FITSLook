use ndarray::Array2;

// ---------------------------------------------------------------------------
// ZScale algorithm (simplified port of the IRAF / Astropy implementation)
// ---------------------------------------------------------------------------

/// Compute ZScale display limits for a 2D image.
/// Returns (z1, z2) — the suggested vmin/vmax for display.
pub fn zscale_limits(data: &Array2<f64>) -> (f64, f64) {
    let flat: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    if flat.is_empty() {
        return (0.0, 1.0);
    }
    let mut sorted = flat.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();

    // Trim outliers (use 2.5th and 97.5th percentiles as starting bounds)
    let lo = sorted[(n as f64 * 0.025) as usize];
    let hi = sorted[((n as f64 * 0.975) as usize).min(n - 1)];

    // Try to do a linear fit on the sorted central pixels
    let contrast = 0.25;
    let n_samples = n.min(1000);
    let step = n as f64 / n_samples as f64;

    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_xx = 0.0_f64;
    let mut sum_xy = 0.0_f64;
    let ns = n_samples as f64;

    for i in 0..n_samples {
        let idx = (i as f64 * step) as usize;
        let x = i as f64;
        let y = sorted[idx.min(n - 1)];
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_xy += x * y;
    }

    let median = sorted[n / 2];
    let denom = ns * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-10 {
        return (lo, hi);
    }
    let slope = (ns * sum_xy - sum_x * sum_y) / denom;

    let z1 = median - (slope / contrast) * (ns / 2.0);
    let z2 = median + (slope / contrast) * (ns / 2.0);

    // Clamp to data range
    let data_min = sorted[0];
    let data_max = sorted[n - 1];
    (z1.max(data_min), z2.min(data_max))
}

// ---------------------------------------------------------------------------
// Normalization functions
// ---------------------------------------------------------------------------

/// Normalizes a value from [vmin, vmax] to [0, 1].
#[inline]
fn linear_norm(val: f64, vmin: f64, vmax: f64) -> f64 {
    if (vmax - vmin).abs() < 1e-15 {
        return 0.5;
    }
    ((val - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
}

/// Apply normalization based on the requested algorithm.
pub fn normalize(val: f64, vmin: f64, vmax: f64, algo: &str) -> f64 {
    let t = linear_norm(val, vmin, vmax);
    match algo {
        "Log" => {
            let a = 1000.0;
            (1.0 + a * t).ln() / (1.0 + a).ln()
        }
        "Sqrt" => t.sqrt(),
        "Asinh" => {
            let a = 10.0;
            (a * t).asinh() / a.asinh()
        }
        "Power" => t.powf(2.0),
        _ => t, // Linear
    }
}

// ---------------------------------------------------------------------------
// Colormap application
// ---------------------------------------------------------------------------

/// Names of available colormaps.
pub const COLORMAP_NAMES: [&str; 5] = ["magma", "viridis", "inferno", "gray", "plasma"];

/// Map a normalized value [0,1] to an RGBA pixel using the named colormap.
pub fn apply_colormap(t: f64, cmap_name: &str) -> [u8; 4] {
    let t_clamped = t.clamp(0.0, 1.0);

    if cmap_name == "gray" {
        let v = (t_clamped * 255.0) as u8;
        return [v, v, v, 255];
    }

    let gradient = match cmap_name {
        "magma" => colorous::MAGMA,
        "inferno" => colorous::INFERNO,
        "plasma" => colorous::PLASMA,
        _ => colorous::VIRIDIS, // default / "viridis"
    };

    let color = gradient.eval_continuous(t_clamped);
    [color.r, color.g, color.b, 255]
}

// ---------------------------------------------------------------------------
// Render a 2D array to RGBA pixels
// ---------------------------------------------------------------------------

/// Convert a 2D floating-point array into an RGBA pixel buffer.
/// Returns (width, height, pixels) where pixels is RGBA in row order, top-to-bottom.
pub fn render_to_rgba(
    data: &Array2<f64>,
    vmin: f64,
    vmax: f64,
    algo: &str,
    cmap_name: &str,
) -> (usize, usize, Vec<u8>) {
    let (ny, nx) = (data.nrows(), data.ncols());
    let mut pixels = vec![0u8; nx * ny * 4];

    for row in 0..ny {
        // Flip vertically (origin='lower' like matplotlib)
        let src_row = ny - 1 - row;
        for col in 0..nx {
            let val = data[[src_row, col]];
            let color = if val.is_nan() {
                [0, 0, 0, 255]
            } else {
                let norm = normalize(val, vmin, vmax, algo);
                apply_colormap(norm, cmap_name)
            };
            let idx = (row * nx + col) * 4;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
            pixels[idx + 3] = color[3];
        }
    }

    (nx, ny, pixels)
}
