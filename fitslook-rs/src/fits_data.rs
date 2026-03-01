use byteorder::{BigEndian, ReadBytesExt};
use ndarray::{Array2, Array3};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Represents one HDU (Header-Data Unit) in a FITS file.
#[derive(Clone)]
pub struct HduInfo {
    pub index: usize,
    pub name: String,
    pub is_image: bool,
    pub shape: Vec<usize>,
    pub header_text: String,
    pub header_cards: BTreeMap<String, String>,
}

/// Pixel data for an image HDU.
#[derive(Clone)]
pub enum ImageData {
    Image2D(Array2<f64>),
    Cube3D(Array3<f64>),
    Spectrum1D(Vec<f64>),
}

/// Data for a table HDU.
#[derive(Clone)]
pub struct TableData {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

/// Holds the fully-parsed contents of a FITS file.
pub struct FitsDocument {
    pub hdus: Vec<HduInfo>,
    pub images: BTreeMap<usize, ImageData>,
    pub tables: BTreeMap<usize, TableData>,
}

// ---------------------------------------------------------------------------
// FITS parser – works directly from bytes (no C dependency)
// ---------------------------------------------------------------------------

const BLOCK_SIZE: usize = 2880;

impl FitsDocument {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let file = File::open(path.as_ref()).map_err(|e| format!("Cannot open file: {e}"))?;
        let mut reader = BufReader::new(file);

        let mut hdus = Vec::new();
        let mut images = BTreeMap::new();
        let mut tables = BTreeMap::new();
        let mut hdu_index: usize = 0;

        loop {
            // ---- Read header blocks ----
            let _header_start = reader
                .stream_position()
                .map_err(|e| format!("Seek error: {e}"))?;

            let mut header_text = String::new();
            let mut cards: BTreeMap<String, String> = BTreeMap::new();
            let mut header_ended = false;

            loop {
                let mut block = [0u8; BLOCK_SIZE];
                // Read a full FITS block (2880 bytes) — loop to handle partial reads
                let mut filled = 0;
                while filled < BLOCK_SIZE {
                    match reader.read(&mut block[filled..]) {
                        Ok(0) => break,  // EOF
                        Ok(n) => filled += n,
                        Err(e) => return Err(format!("Read error: {e}")),
                    }
                }
                if filled == 0 {
                    // EOF before END card – we're done
                    if hdu_index == 0 {
                        return Err("Empty or invalid FITS file".into());
                    }
                    header_ended = true;
                    break;
                }
                // Pad remainder with spaces if short block (end of file)
                for byte in block.iter_mut().skip(filled) {
                    *byte = b' ';
                }
                // Parse 80-byte cards
                for card_idx in 0..(BLOCK_SIZE / 80) {
                    let start = card_idx * 80;
                    let card_bytes = &block[start..start + 80];

                    // Skip cards that contain non-ASCII bytes (binary data, not a header)
                    let is_ascii = card_bytes.iter().all(|&b| b.is_ascii());
                    if !is_ascii {
                        // If we're still in a header and hit binary, something is wrong
                        // — just skip this card
                        continue;
                    }

                    let card_str = String::from_utf8_lossy(card_bytes).trim_end().to_string();
                    if !card_str.is_empty() {
                        header_text.push_str(&card_str);
                        header_text.push('\n');
                    }
                    let keyword = card_str
                        .get(..8)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if keyword == "END" {
                        header_ended = true;
                        break;
                    }
                    if card_str.len() > 10 {
                        if let (Some(&eq), Some(value_part)) = (card_str.as_bytes().get(8), card_str.get(10..)) {
                            if eq == b'=' {
                                let value = value_part
                                    .split('/')
                                    .next()
                                    .unwrap_or("")
                                    .trim()
                                    .trim_matches('\'')
                                    .trim()
                                    .to_string();
                                cards.insert(keyword, value);
                            }
                        }
                    }
                }
                if header_ended {
                    break;
                }
            }

            if hdu_index == 0 && cards.is_empty() {
                return Err("Could not parse FITS header".into());
            }
            if cards.is_empty() && header_ended {
                break; // EOF
            }

            // ---- Parse shape / type ----
            let bitpix: i32 = cards
                .get("BITPIX")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let naxis: usize = cards
                .get("NAXIS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let mut shape = Vec::new();
            for i in 1..=naxis {
                let key = format!("NAXIS{i}");
                let dim: usize = cards.get(&key).and_then(|v| v.parse().ok()).unwrap_or(0);
                shape.push(dim);
            }

            let xtension = cards
                .get("XTENSION")
                .cloned()
                .unwrap_or_default()
                .to_uppercase();
            let is_image = xtension.is_empty()
                || xtension.contains("IMAGE")
                || (hdu_index == 0 && !xtension.contains("TABLE"));

            let ext_name = cards
                .get("EXTNAME")
                .cloned()
                .unwrap_or_else(|| {
                    if hdu_index == 0 {
                        "PRIMARY".to_string()
                    } else {
                        format!("EXT_{hdu_index}")
                    }
                });

            let hdu_info = HduInfo {
                index: hdu_index,
                name: ext_name,
                is_image,
                shape: shape.clone(),
                header_text: header_text.clone(),
                header_cards: cards.clone(),
            };

            // ---- Compute data size & read data ----
            let total_pixels: usize = shape.iter().product();
            let bytes_per_pixel = (bitpix.unsigned_abs() as usize) / 8;
            let data_bytes = total_pixels * bytes_per_pixel;

            // Align to FITS block boundary
            let data_blocks = (data_bytes + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let padded_data_bytes = data_blocks * BLOCK_SIZE;

            if is_image && naxis > 0 && total_pixels > 0 {
                // Read image data
                let mut raw = vec![0u8; data_bytes];
                reader.read_exact(&mut raw).map_err(|e| format!("Data read error: {e}"))?;
                // Skip padding
                let pad = padded_data_bytes - data_bytes;
                if pad > 0 {
                    reader.seek(SeekFrom::Current(pad as i64)).ok();
                }

                let pixels = read_pixels(&raw, bitpix, total_pixels);
                let bscale: f64 = cards.get("BSCALE").and_then(|v| v.parse().ok()).unwrap_or(1.0);
                let bzero: f64 = cards.get("BZERO").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let pixels: Vec<f64> = pixels.iter().map(|&v| {
                    let val = v * bscale + bzero;
                    if val.is_nan() || val.is_infinite() { 0.0 } else { val }
                }).collect();

                // FITS stores data in FORTRAN order (first axis varies fastest in memory)
                // shape = [NAXIS1, NAXIS2, ...] where NAXIS1 is the fastest-varying axis
                let img_data = if shape.len() == 1 {
                    ImageData::Spectrum1D(pixels)
                } else if shape.len() == 2 {
                    let (nx, ny) = (shape[0], shape[1]);
                    // Data is stored row-major from FITS perspective: ny rows of nx columns
                    let arr = Array2::from_shape_vec((ny, nx), pixels)
                        .map_err(|e| format!("Shape error: {e}"))?;
                    ImageData::Image2D(arr)
                } else if shape.len() >= 3 {
                    let (nx, ny, nz) = (shape[0], shape[1], shape[2]);
                    // For cubes: nz frames of ny×nx images
                    let arr = Array3::from_shape_vec((nz, ny, nx), pixels)
                        .map_err(|e| format!("Shape error: {e}"))?;
                    ImageData::Cube3D(arr)
                } else {
                    ImageData::Spectrum1D(pixels)
                };
                images.insert(hdu_index, img_data);
            } else if !is_image && naxis > 0 && data_bytes > 0 {
                // Table – we read raw bytes, try to parse column names from TFORMn / TTYPEn
                let mut raw = vec![0u8; data_bytes];
                reader.read_exact(&mut raw).map_err(|e| format!("Table read error: {e}"))?;
                let pad = padded_data_bytes - data_bytes;
                if pad > 0 {
                    reader.seek(SeekFrom::Current(pad as i64)).ok();
                }

                let tfields: usize = cards.get("TFIELDS").and_then(|v| v.parse().ok()).unwrap_or(0);
                let nrows: usize = cards.get("NAXIS2").and_then(|v| v.parse().ok()).unwrap_or(0);

                let mut columns = Vec::new();
                let mut col_formats = Vec::new();
                for i in 1..=tfields {
                    let name = cards.get(&format!("TTYPE{i}")).cloned().unwrap_or_else(|| format!("COL_{i}"));
                    columns.push(name);
                    let fmt = cards.get(&format!("TFORM{i}")).cloned().unwrap_or_default();
                    col_formats.push(fmt);
                }

                // Parse binary or ASCII table rows
                let is_bintable = xtension.contains("BINTABLE");
                let row_bytes: usize = cards.get("NAXIS1").and_then(|v| v.parse().ok()).unwrap_or(0);

                let mut rows: Vec<Vec<String>> = Vec::new();
                let max_rows = nrows.min(500); // Limit for display

                if is_bintable && row_bytes > 0 {
                    // Parse binary table
                    let col_sizes = parse_bintable_col_sizes(&col_formats);
                    for r in 0..max_rows {
                        let row_start = r * row_bytes;
                        let mut row_vals = Vec::new();
                        let mut offset = row_start;
                        for (_c, (fmt, size)) in col_formats.iter().zip(&col_sizes).enumerate() {
                            if offset + size > raw.len() {
                                row_vals.push("?".to_string());
                                offset += size;
                                continue;
                            }
                            let val = read_bintable_cell(&raw[offset..offset + size], fmt);
                            row_vals.push(val);
                            offset += size;
                        }
                        rows.push(row_vals);
                    }
                } else if row_bytes > 0 {
                    // ASCII table
                    for r in 0..max_rows {
                        let start = r * row_bytes;
                        let end = (start + row_bytes).min(raw.len());
                        let line = String::from_utf8_lossy(&raw[start..end]).to_string();
                        // Split by column width — simplified: just split by whitespace
                        let vals: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
                        rows.push(vals);
                    }
                }

                tables.insert(hdu_index, TableData { columns, rows });
            } else {
                // Skip data block
                if padded_data_bytes > 0 {
                    reader.seek(SeekFrom::Current(padded_data_bytes as i64)).ok();
                }
            }

            hdus.push(hdu_info);
            hdu_index += 1;

            // Check if we've reached EOF
            let mut peek = [0u8; 1];
            match reader.read(&mut peek) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    reader.seek(SeekFrom::Current(-1)).ok();
                }
                Err(_) => break,
            }
        }

        Ok(FitsDocument {
            hdus,
            images,
            tables,
        })
    }
}

/// Read pixel values from raw bytes based on BITPIX.
fn read_pixels(raw: &[u8], bitpix: i32, count: usize) -> Vec<f64> {
    let mut cursor = std::io::Cursor::new(raw);
    let mut pixels = Vec::with_capacity(count);
    for _ in 0..count {
        let val: f64 = match bitpix {
            8 => cursor.read_u8().unwrap_or(0) as f64,
            16 => cursor.read_i16::<BigEndian>().unwrap_or(0) as f64,
            32 => cursor.read_i32::<BigEndian>().unwrap_or(0) as f64,
            64 => cursor.read_i64::<BigEndian>().unwrap_or(0) as f64,
            -32 => cursor.read_f32::<BigEndian>().unwrap_or(0.0) as f64,
            -64 => cursor.read_f64::<BigEndian>().unwrap_or(0.0),
            _ => 0.0,
        };
        pixels.push(val);
    }
    pixels
}

/// Parse BINTABLE column sizes from TFORM strings.
fn parse_bintable_col_sizes(formats: &[String]) -> Vec<usize> {
    formats
        .iter()
        .map(|fmt| {
            let fmt = fmt.trim();
            if fmt.is_empty() {
                return 0;
            }
            // e.g. "1E", "1D", "20A", "1J", "1K", etc.
            let (repeat_str, type_char) = if fmt.len() == 1 {
                ("1", fmt.chars().next().unwrap())
            } else {
                let last = fmt.chars().last().unwrap();
                (&fmt[..fmt.len() - 1], last)
            };
            let repeat: usize = repeat_str.parse().unwrap_or(1);
            let elem_size = match type_char.to_ascii_uppercase() {
                'L' | 'B' | 'A' | 'X' => 1,
                'I' => 2,
                'J' | 'E' => 4,
                'K' | 'D' | 'C' => 8,
                'M' => 16,
                _ => 4,
            };
            repeat * elem_size
        })
        .collect()
}

/// Read a single cell value from a binary table.
fn read_bintable_cell(bytes: &[u8], fmt: &str) -> String {
    let fmt = fmt.trim();
    if fmt.is_empty() || bytes.is_empty() {
        return String::new();
    }
    let type_char = fmt.chars().last().unwrap().to_ascii_uppercase();
    let repeat_str = if fmt.len() > 1 { &fmt[..fmt.len() - 1] } else { "1" };
    let repeat: usize = repeat_str.parse().unwrap_or(1);

    match type_char {
        'A' => {
            // ASCII string
            String::from_utf8_lossy(bytes).trim().to_string()
        }
        'E' => {
            // 32-bit float
            let mut cursor = std::io::Cursor::new(bytes);
            let mut vals = Vec::new();
            for _ in 0..repeat {
                if let Ok(v) = cursor.read_f32::<BigEndian>() {
                    vals.push(format!("{:.6}", v));
                }
            }
            vals.join(", ")
        }
        'D' => {
            // 64-bit float
            let mut cursor = std::io::Cursor::new(bytes);
            let mut vals = Vec::new();
            for _ in 0..repeat {
                if let Ok(v) = cursor.read_f64::<BigEndian>() {
                    vals.push(format!("{:.6}", v));
                }
            }
            vals.join(", ")
        }
        'J' => {
            // 32-bit int
            let mut cursor = std::io::Cursor::new(bytes);
            let mut vals = Vec::new();
            for _ in 0..repeat {
                if let Ok(v) = cursor.read_i32::<BigEndian>() {
                    vals.push(format!("{}", v));
                }
            }
            vals.join(", ")
        }
        'K' => {
            // 64-bit int
            let mut cursor = std::io::Cursor::new(bytes);
            let mut vals = Vec::new();
            for _ in 0..repeat {
                if let Ok(v) = cursor.read_i64::<BigEndian>() {
                    vals.push(format!("{}", v));
                }
            }
            vals.join(", ")
        }
        'I' => {
            // 16-bit int
            let mut cursor = std::io::Cursor::new(bytes);
            let mut vals = Vec::new();
            for _ in 0..repeat {
                if let Ok(v) = cursor.read_i16::<BigEndian>() {
                    vals.push(format!("{}", v));
                }
            }
            vals.join(", ")
        }
        'B' => {
            // unsigned byte
            let mut vals = Vec::new();
            for &b in bytes.iter().take(repeat) {
                vals.push(format!("{}", b));
            }
            vals.join(", ")
        }
        'L' => {
            // logical
            let mut vals = Vec::new();
            for &b in bytes.iter().take(repeat) {
                vals.push(if b == b'T' { "T" } else { "F" }.to_string());
            }
            vals.join(", ")
        }
        _ => format!("<{} bytes>", bytes.len()),
    }
}
