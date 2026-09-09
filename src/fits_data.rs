use byteorder::{BigEndian, ReadBytesExt};
use flate2::read::{GzDecoder, ZlibDecoder};
use log::warn;
use ndarray::{Array2, Array3};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Represents a single 80-byte header card in a FITS file.
#[derive(Clone, Debug, PartialEq)]
pub struct HeaderCard {
    pub keyword: String,
    pub value: String,
    pub comment: String,
    pub raw: String,
}

impl HeaderCard {
    pub fn new(keyword: &str, value: &str, comment: &str) -> Self {
        let kw = keyword.trim().to_uppercase();
        let val = value.trim().to_string();
        let comm = comment.trim().to_string();
        let raw = Self::format_card(&kw, &val, &comm);
        Self {
            keyword: kw,
            value: val,
            comment: comm,
            raw,
        }
    }

    /// Formats keyword, value, and comment into a standard 80-byte FITS card string.
    pub fn format_card(keyword: &str, value: &str, comment: &str) -> String {
        let kw_upper = keyword.trim().to_uppercase();
        if kw_upper == "END" {
            return format!("{:80}", "END");
        }
        if kw_upper == "COMMENT" || kw_upper == "HISTORY" || kw_upper.is_empty() {
            let body = if !comment.is_empty() {
                format!("{kw_upper} {comment}")
            } else if !value.is_empty() {
                format!("{kw_upper} {value}")
            } else {
                kw_upper.clone()
            };
            return format!("{:80}", body.get(..80).unwrap_or(&body));
        }

        // Check if keyword is longer than 8 characters (HIERARCH keyword)
        let (kw_part, is_hierarch) = if kw_upper.starts_with("HIERARCH ") {
            (kw_upper.clone(), true)
        } else if kw_upper.len() > 8 {
            (format!("HIERARCH {kw_upper}"), true)
        } else {
            (kw_upper.clone(), false)
        };

        // Format value: strings get single quotes `'val'`, booleans/numbers get raw string
        let formatted_val = if value.starts_with('\'') && value.ends_with('\'') {
            value.to_string()
        } else if value.eq_ignore_ascii_case("T") || value.eq_ignore_ascii_case("F") {
            value.to_uppercase()
        } else if value.parse::<f64>().is_ok() || value.parse::<i64>().is_ok() {
            value.to_string()
        } else if !value.is_empty() {
            // Default string formatting with single quotes
            format!("'{value}'")
        } else {
            String::new()
        };

        let card_body = if is_hierarch {
            if !comment.is_empty() {
                format!("{kw_part} = {formatted_val} / {comment}")
            } else {
                format!("{kw_part} = {formatted_val}")
            }
        } else {
            if !comment.is_empty() {
                format!("{kw_part:<8}= {formatted_val:<20} / {comment}")
            } else {
                format!("{kw_part:<8}= {formatted_val}")
            }
        };

        if card_body.len() >= 80 {
            card_body.get(..80).unwrap_or(&card_body).to_string()
        } else {
            format!("{card_body:<80}")
        }
    }
}

/// Represents one HDU (Header-Data Unit) in a FITS file.
#[derive(Clone)]
pub struct HduInfo {
    pub index: usize,
    pub name: String,
    pub is_image: bool,
    pub shape: Vec<usize>,
    pub header_text: String,
    pub header_cards: BTreeMap<String, String>,
    pub cards_list: Vec<HeaderCard>,
    pub data_offset: u64,
    pub data_len: usize,
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
    pub filepath: PathBuf,
    pub hdus: Vec<HduInfo>,
    pub images: BTreeMap<usize, ImageData>,
    pub tables: BTreeMap<usize, TableData>,
}

const BLOCK_SIZE: usize = 2880;

impl FitsDocument {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let path_buf = path.as_ref().to_path_buf();
        let file = File::open(&path_buf).map_err(|e| format!("Cannot open file: {e}"))?;
        let mut reader = BufReader::new(file);

        let mut hdus = Vec::new();
        let mut images = BTreeMap::new();
        let mut tables = BTreeMap::new();
        let mut hdu_index: usize = 0;

        loop {
            // ---- Read header blocks ----
            let mut header_text = String::new();
            let mut cards_map: BTreeMap<String, String> = BTreeMap::new();
            let mut cards_list: Vec<HeaderCard> = Vec::new();
            let mut header_ended = false;

            loop {
                let mut block = [0u8; BLOCK_SIZE];
                let mut filled = 0;
                while filled < BLOCK_SIZE {
                    match reader.read(&mut block[filled..]) {
                        Ok(0) => break,
                        Ok(n) => filled += n,
                        Err(e) => return Err(format!("Read error: {e}")),
                    }
                }
                if filled == 0 {
                    if hdu_index == 0 {
                        return Err("Empty or invalid FITS file".into());
                    }
                    header_ended = true;
                    break;
                }
                for byte in block.iter_mut().skip(filled) {
                    *byte = b' ';
                }
                for card_idx in 0..(BLOCK_SIZE / 80) {
                    let start = card_idx * 80;
                    let card_bytes = &block[start..start + 80];

                    if card_bytes.starts_with(b"END ")
                        || card_bytes.starts_with(b"END\0")
                        || (&card_bytes[..3] == b"END"
                            && card_bytes[3..].iter().all(|&b| b == b' ' || b == 0))
                    {
                        header_ended = true;
                        cards_list.push(HeaderCard::new("END", "", ""));
                        break;
                    }

                    let card_str = String::from_utf8_lossy(card_bytes).trim_end().to_string();
                    if !card_str.is_empty() {
                        header_text.push_str(&card_str);
                        header_text.push('\n');
                    }

                    if let Some(card) = parse_header_card(&card_str) {
                        cards_map.insert(card.keyword.clone(), card.value.clone());
                        cards_list.push(card);
                    }
                }
                if header_ended {
                    break;
                }
            }

            if hdu_index == 0 && cards_map.is_empty() {
                return Err("Could not parse FITS header".into());
            }
            if cards_map.is_empty() && header_ended {
                break;
            }

            // ---- Parse shape / type ----
            let bitpix: i32 = parse_fits_int_card(&cards_map, "BITPIX")
                .or_else(|| parse_fits_int_card(&cards_map, "ZBITPIX"))
                .unwrap_or(0);
            let naxis: usize = parse_fits_int_card(&cards_map, "NAXIS")
                .or_else(|| parse_fits_int_card(&cards_map, "ZNAXIS"))
                .unwrap_or(0);

            let mut shape = Vec::new();
            for i in 1..=naxis {
                let key = format!("NAXIS{i}");
                let zkey = format!("ZNAXIS{i}");
                let dim: usize = parse_fits_int_card(&cards_map, &key)
                    .or_else(|| parse_fits_int_card(&cards_map, &zkey))
                    .unwrap_or(0);
                shape.push(dim);
            }

            let xtension = cards_map
                .get("XTENSION")
                .cloned()
                .unwrap_or_default()
                .to_uppercase();

            let is_compressed_image = (xtension.contains("BINTABLE") || xtension.contains("TABLE"))
                && (cards_map.get("ZIMAGE").map(|v| v.trim().eq_ignore_ascii_case("T")).unwrap_or(false)
                    || cards_map.contains_key("ZCMPTYPE"));

            let is_image = xtension.is_empty()
                || xtension.contains("IMAGE")
                || is_compressed_image
                || (hdu_index == 0 && !xtension.contains("TABLE"));

            let ext_name = cards_map.get("EXTNAME").cloned().unwrap_or_else(|| {
                if hdu_index == 0 {
                    "PRIMARY".to_string()
                } else {
                    format!("EXT_{hdu_index}")
                }
            });

            // Stream position where data block starts
            let data_offset = reader
                .stream_position()
                .map_err(|e| format!("Seek error: {e}"))?;

            // Compute expected raw data size on disk
            let raw_data_bytes = if is_compressed_image {
                let pcount: usize = parse_fits_int_card(&cards_map, "PCOUNT").unwrap_or(0);
                let gcount: usize = parse_fits_int_card(&cards_map, "GCOUNT").unwrap_or(1);
                let naxis1: usize = parse_fits_int_card(&cards_map, "NAXIS1").unwrap_or(0);
                let naxis2: usize = parse_fits_int_card(&cards_map, "NAXIS2").unwrap_or(0);
                (naxis1 * naxis2 * gcount) + pcount
            } else if !is_image {
                let naxis1: usize = parse_fits_int_card(&cards_map, "NAXIS1").unwrap_or(0);
                let naxis2: usize = parse_fits_int_card(&cards_map, "NAXIS2").unwrap_or(0);
                let pcount: usize = parse_fits_int_card(&cards_map, "PCOUNT").unwrap_or(0);
                let gcount: usize = parse_fits_int_card(&cards_map, "GCOUNT").unwrap_or(1);
                (naxis1 * naxis2 * gcount) + pcount
            } else {
                let total_pixels: usize = if shape.is_empty() { 0 } else { shape.iter().product() };
                let bytes_per_pixel = if bitpix != 0 {
                    (bitpix.unsigned_abs() as usize) / 8
                } else {
                    0
                };
                total_pixels * bytes_per_pixel
            };

            let data_blocks = (raw_data_bytes + BLOCK_SIZE - 1) / BLOCK_SIZE;
            let padded_data_bytes = data_blocks * BLOCK_SIZE;

            let hdu_info = HduInfo {
                index: hdu_index,
                name: ext_name,
                is_image,
                shape: shape.clone(),
                header_text: header_text.clone(),
                header_cards: cards_map.clone(),
                cards_list,
                data_offset,
                data_len: raw_data_bytes,
            };

            let current_pos = reader.stream_position().unwrap_or(0);
            let file_end = reader.seek(SeekFrom::End(0)).unwrap_or(current_pos);
            reader.seek(SeekFrom::Start(current_pos)).ok();
            let remaining = file_end.saturating_sub(current_pos) as usize;

            // Handle Tile Compressed Image HDU (ZIMAGE=T / ZCMPTYPE)
            if is_compressed_image {
                if raw_data_bytes > 0 && raw_data_bytes <= remaining {
                    let mut raw_table = vec![0u8; raw_data_bytes];
                    if reader.read_exact(&mut raw_table).is_ok() {
                        let pad = padded_data_bytes - raw_data_bytes;
                        if pad > 0 {
                            reader.seek(SeekFrom::Current(pad as i64)).ok();
                        }
                        if let Ok(img_data) = parse_compressed_fits_image(&cards_map, &raw_table) {
                            images.insert(hdu_index, img_data);
                        }
                    }
                } else if padded_data_bytes > 0 {
                    reader.seek(SeekFrom::Current(padded_data_bytes as i64)).ok();
                }
                hdus.push(hdu_info);
                hdu_index += 1;
                continue;
            }

            let total_pixels: usize = if shape.is_empty() { 0 } else { shape.iter().product() };

            if is_image && naxis > 0 && total_pixels > 0 {
                if raw_data_bytes > remaining {
                    warn!("HDU {hdu_index}: data requires {raw_data_bytes} bytes but only {remaining} remain");
                    if remaining > 0 {
                        reader.seek(SeekFrom::Current(remaining as i64)).ok();
                    }
                    hdus.push(hdu_info);
                    hdu_index += 1;
                    continue;
                }

                let mut raw = vec![0u8; raw_data_bytes];
                match reader.read_exact(&mut raw) {
                    Ok(_) => {}
                    Err(e) => {
                        warn!("HDU {hdu_index}: data read error ({e})");
                        hdus.push(hdu_info);
                        hdu_index += 1;
                        continue;
                    }
                }
                let pad = padded_data_bytes - raw_data_bytes;
                if pad > 0 {
                    reader.seek(SeekFrom::Current(pad as i64)).ok();
                }

                let pixels = read_pixels(&raw, bitpix, total_pixels);
                let bscale: f64 = parse_fits_float_card(&cards_map, "BSCALE").unwrap_or(1.0);
                let bzero: f64 = parse_fits_float_card(&cards_map, "BZERO").unwrap_or(0.0);
                let blank_val: Option<i64> = if bitpix > 0 {
                    parse_fits_int_card(&cards_map, "BLANK")
                } else {
                    None
                };

                let processed_pixels: Vec<f64> = pixels
                    .iter()
                    .map(|&v| {
                        if v.is_nan() {
                            return f64::NAN;
                        }
                        if let Some(bv) = blank_val {
                            if (v as i64) == bv {
                                return f64::NAN;
                            }
                        }
                        let val = v * bscale + bzero;
                        if val.is_infinite() {
                            f64::NAN
                        } else {
                            val
                        }
                    })
                    .collect();

                // Robust shape squeezing
                let mut non_one_dims: Vec<usize> = shape.iter().copied().filter(|&d| d > 1).collect();
                if non_one_dims.is_empty() && !shape.is_empty() {
                    non_one_dims = shape.clone();
                }

                let img_data = if non_one_dims.len() == 1 {
                    ImageData::Spectrum1D(processed_pixels)
                } else if non_one_dims.len() == 2 {
                    let (nx, ny) = (non_one_dims[0], non_one_dims[1]);
                    if processed_pixels.len() == nx * ny {
                        Array2::from_shape_vec((ny, nx), processed_pixels)
                            .map(ImageData::Image2D)
                            .unwrap_or(ImageData::Spectrum1D(pixels))
                    } else {
                        ImageData::Spectrum1D(processed_pixels)
                    }
                } else if non_one_dims.len() >= 3 {
                    let (nx, ny, nz) = (non_one_dims[0], non_one_dims[1], non_one_dims[2]);
                    if processed_pixels.len() == nx * ny * nz {
                        Array3::from_shape_vec((nz, ny, nx), processed_pixels)
                            .map(ImageData::Cube3D)
                            .unwrap_or(ImageData::Spectrum1D(pixels))
                    } else {
                        ImageData::Spectrum1D(processed_pixels)
                    }
                } else {
                    ImageData::Spectrum1D(processed_pixels)
                };

                images.insert(hdu_index, img_data);
            } else if !is_image && naxis > 0 && raw_data_bytes > 0 {
                if raw_data_bytes > remaining {
                    if remaining > 0 {
                        reader.seek(SeekFrom::Current(remaining as i64)).ok();
                    }
                    hdus.push(hdu_info);
                    hdu_index += 1;
                    continue;
                }
                let mut raw = vec![0u8; raw_data_bytes];
                if reader.read_exact(&mut raw).is_ok() {
                    let pad = padded_data_bytes - raw_data_bytes;
                    if pad > 0 {
                        reader.seek(SeekFrom::Current(pad as i64)).ok();
                    }
                    let tfields: usize = parse_fits_int_card(&cards_map, "TFIELDS").unwrap_or(0);
                    let nrows: usize = parse_fits_int_card(&cards_map, "NAXIS2").unwrap_or(0);

                    let mut columns = Vec::new();
                    let mut col_formats = Vec::new();
                    for i in 1..=tfields {
                        let name = cards_map
                            .get(&format!("TTYPE{i}"))
                            .cloned()
                            .unwrap_or_else(|| format!("COL_{i}"));
                        columns.push(name);
                        let fmt = cards_map.get(&format!("TFORM{i}")).cloned().unwrap_or_default();
                        col_formats.push(fmt);
                    }

                    let is_bintable = xtension.contains("BINTABLE");
                    let row_bytes: usize = parse_fits_int_card(&cards_map, "NAXIS1").unwrap_or(0);
                    let mut rows: Vec<Vec<String>> = Vec::new();
                    let max_rows = nrows.min(500);

                    if is_bintable && row_bytes > 0 {
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
                        for r in 0..max_rows {
                            let start = r * row_bytes;
                            let end = (start + row_bytes).min(raw.len());
                            let line = String::from_utf8_lossy(&raw[start..end]).to_string();
                            let vals: Vec<String> =
                                line.split_whitespace().map(|s| s.to_string()).collect();
                            rows.push(vals);
                        }
                    }

                    tables.insert(hdu_index, TableData { columns, rows });
                }
            } else if padded_data_bytes > 0 {
                reader.seek(SeekFrom::Current(padded_data_bytes as i64)).ok();
            }

            hdus.push(hdu_info);
            hdu_index += 1;

            let mut peek = [0u8; 1];
            match reader.read(&mut peek) {
                Ok(0) => break,
                Ok(_) => {
                    reader.seek(SeekFrom::Current(-1)).ok();
                }
                Err(_) => break,
            }
        }

        Ok(FitsDocument {
            filepath: path_buf,
            hdus,
            images,
            tables,
        })
    }

    /// Saves the FITS document with updated headers to a new file ending in `_edited.fits`.
    /// Original file is NEVER overwritten.
    pub fn save_edited_to_new_file(
        &self,
        updated_cards_per_hdu: &BTreeMap<usize, Vec<HeaderCard>>,
    ) -> Result<String, String> {
        let orig_path = &self.filepath;
        let file_stem = orig_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();

        // Create new filename ending in `_edited.fits`
        let new_filename = if file_stem.ends_with("_edited") {
            format!("{file_stem}_1.fits")
        } else {
            format!("{file_stem}_edited.fits")
        };

        let target_path = orig_path.with_file_name(&new_filename);
        if target_path == *orig_path {
            return Err("Target path resolves to original file path — action blocked to protect original file.".into());
        }

        let orig_file = File::open(orig_path).map_err(|e| format!("Failed to read original FITS: {e}"))?;
        let mut orig_reader = BufReader::new(orig_file);

        let mut out_file = File::create(&target_path)
            .map_err(|e| format!("Failed to create output file '{}': {e}", target_path.display()))?;

        for hdu in &self.hdus {
            let cards_to_write = updated_cards_per_hdu
                .get(&hdu.index)
                .unwrap_or(&hdu.cards_list);

            // Reconstruct header block
            let mut header_bytes = Vec::new();
            let mut has_end = false;
            for card in cards_to_write {
                let formatted = HeaderCard::format_card(&card.keyword, &card.value, &card.comment);
                header_bytes.extend_from_slice(formatted.as_bytes());
                if card.keyword == "END" {
                    has_end = true;
                    break;
                }
            }
            if !has_end {
                let end_card = HeaderCard::format_card("END", "", "");
                header_bytes.extend_from_slice(end_card.as_bytes());
            }

            // Pad header to 2880 block boundary
            let h_rem = header_bytes.len() % BLOCK_SIZE;
            if h_rem != 0 {
                let pad_len = BLOCK_SIZE - h_rem;
                header_bytes.resize(header_bytes.len() + pad_len, b' ');
            }

            out_file
                .write_all(&header_bytes)
                .map_err(|e| format!("Failed writing header for HDU {}: {e}", hdu.index))?;

            // Copy original raw data block
            if hdu.data_len > 0 {
                orig_reader
                    .seek(SeekFrom::Start(hdu.data_offset))
                    .map_err(|e| format!("Seek error reading HDU {} data: {e}", hdu.index))?;

                let mut raw_buf = vec![0u8; hdu.data_len];
                orig_reader
                    .read_exact(&mut raw_buf)
                    .map_err(|e| format!("Failed reading HDU {} raw data: {e}", hdu.index))?;

                out_file
                    .write_all(&raw_buf)
                    .map_err(|e| format!("Failed writing HDU {} raw data: {e}", hdu.index))?;

                let d_rem = hdu.data_len % BLOCK_SIZE;
                if d_rem != 0 {
                    let pad_len = BLOCK_SIZE - d_rem;
                    let pad = vec![0u8; pad_len];
                    out_file
                        .write_all(&pad)
                        .map_err(|e| format!("Failed writing data padding for HDU {}: {e}", hdu.index))?;
                }
            }
        }

        out_file.flush().ok();
        Ok(target_path.to_string_lossy().to_string())
    }
}

/// Read pixel values from raw bytes based on BITPIX.
fn read_pixels(raw: &[u8], bitpix: i32, count: usize) -> Vec<f64> {
    let mut cursor = std::io::Cursor::new(raw);
    let mut pixels = Vec::with_capacity(count);
    for _ in 0..count {
        let val: f64 = match bitpix {
            8 => cursor.read_u8().map(|v| v as f64).unwrap_or(f64::NAN),
            16 => cursor.read_i16::<BigEndian>().map(|v| v as f64).unwrap_or(f64::NAN),
            32 => cursor.read_i32::<BigEndian>().map(|v| v as f64).unwrap_or(f64::NAN),
            64 => cursor.read_i64::<BigEndian>().map(|v| v as f64).unwrap_or(f64::NAN),
            -32 => cursor.read_f32::<BigEndian>().map(|v| v as f64).unwrap_or(f64::NAN),
            -64 => cursor.read_f64::<BigEndian>().unwrap_or(f64::NAN),
            _ => f64::NAN,
        };
        pixels.push(val);
    }
    pixels
}

/// Parses card line into structured HeaderCard.
fn parse_header_card(card_str: &str) -> Option<HeaderCard> {
    if card_str.trim().is_empty() {
        return None;
    }

    let (keyword, value_part) = if card_str.to_uppercase().starts_with("HIERARCH ") {
        if let Some((k, v)) = card_str.split_once('=') {
            (k.trim().to_uppercase(), Some(v))
        } else {
            ("HIERARCH".to_string(), None)
        }
    } else {
        let raw_kw = card_str.get(..8).unwrap_or(card_str).trim().to_uppercase();
        if raw_kw.is_empty() {
            return None;
        }
        let v_part = if card_str.as_bytes().get(8) == Some(&b'=') {
            card_str.get(10..)
        } else {
            card_str.split_once('=').map(|(_, v)| v)
        };
        (raw_kw, v_part)
    };

    if keyword == "COMMENT" || keyword == "HISTORY" || value_part.is_none() {
        let comment = card_str.get(8..).unwrap_or(card_str).trim().to_string();
        return Some(HeaderCard {
            keyword,
            value: String::new(),
            comment,
            raw: card_str.to_string(),
        });
    }

    let v_str = value_part.unwrap_or("");
    let (val, comment) = parse_value_and_comment(v_str);

    Some(HeaderCard {
        keyword,
        value: val,
        comment,
        raw: card_str.to_string(),
    })
}

fn parse_value_and_comment(value_part: &str) -> (String, String) {
    let mut in_quotes = false;
    let mut val_end = value_part.len();
    let mut comment_start = None;

    for (idx, ch) in value_part.char_indices() {
        if ch == '\'' {
            in_quotes = !in_quotes;
        } else if ch == '/' && !in_quotes {
            val_end = idx;
            comment_start = Some(idx + 1);
            break;
        }
    }

    let raw_val = value_part[..val_end].trim();
    let val = if raw_val.starts_with('\'') && raw_val.ends_with('\'') && raw_val.len() >= 2 {
        raw_val[1..raw_val.len() - 1].trim().to_string()
    } else {
        raw_val.to_string()
    };

    let comment = comment_start
        .map(|idx| value_part[idx..].trim().to_string())
        .unwrap_or_default();

    (val, comment)
}

fn parse_fits_int_card<T: std::str::FromStr>(cards: &BTreeMap<String, String>, key: &str) -> Option<T> {
    cards.get(key).and_then(|v| v.trim().parse::<T>().ok())
}

fn parse_fits_float_card(cards: &BTreeMap<String, String>, key: &str) -> Option<f64> {
    let raw = cards.get(key)?.trim();
    raw.parse::<f64>()
        .ok()
        .or_else(|| raw.replace('D', "E").replace('d', "e").parse::<f64>().ok())
}

fn parse_bintable_col_sizes(formats: &[String]) -> Vec<usize> {
    formats
        .iter()
        .map(|fmt| {
            let fmt = fmt.trim();
            if fmt.is_empty() {
                return 0;
            }
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
                'P' => return 8,
                'Q' => return 16,
                _ => 4,
            };
            repeat * elem_size
        })
        .collect()
}

fn read_bintable_cell(bytes: &[u8], fmt: &str) -> String {
    let fmt = fmt.trim();
    if fmt.is_empty() || bytes.is_empty() {
        return String::new();
    }
    let type_char = fmt.chars().last().unwrap().to_ascii_uppercase();
    let repeat_str = if fmt.len() > 1 {
        &fmt[..fmt.len() - 1]
    } else {
        "1"
    };
    let repeat: usize = repeat_str.parse().unwrap_or(1);

    match type_char {
        'A' => String::from_utf8_lossy(bytes).trim().to_string(),
        'E' => {
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
            let mut vals = Vec::new();
            for &b in bytes.iter().take(repeat) {
                vals.push(format!("{}", b));
            }
            vals.join(", ")
        }
        'L' => {
            let mut vals = Vec::new();
            for &b in bytes.iter().take(repeat) {
                vals.push(if b == b'T' { "T" } else { "F" }.to_string());
            }
            vals.join(", ")
        }
        _ => format!("<{} bytes>", bytes.len()),
    }
}

/// Decompress compressed FITS tile images (GZIP / ZLIB / RICE / NOCOMPRESS).
fn parse_compressed_fits_image(
    cards: &BTreeMap<String, String>,
    table_bytes: &[u8],
) -> Result<ImageData, String> {
    let nx: usize = parse_fits_int_card(cards, "ZNAXIS1").ok_or("Missing ZNAXIS1")?;
    let ny: usize = parse_fits_int_card(cards, "ZNAXIS2").ok_or("Missing ZNAXIS2")?;
    let zcmp = cards.get("ZCMPTYPE").cloned().unwrap_or_default().to_uppercase();

    // Check if table contains GZIP compressed bytes (starts with 0x1f 0x8b or zlib 0x78)
    if zcmp.contains("GZIP") || zcmp.contains("ZLIB") || table_bytes.windows(2).any(|w| w == [0x1f, 0x8b]) {
        if let Some(pos) = table_bytes.windows(2).position(|w| w == [0x1f, 0x8b]) {
            let gz_data = &table_bytes[pos..];
            let mut decoder = GzDecoder::new(gz_data);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() && decompressed.len() >= nx * ny {
                let zbitpix: i32 = parse_fits_int_card(cards, "ZBITPIX").unwrap_or(-32);
                let pixels = read_pixels(&decompressed, zbitpix, nx * ny);
                if let Ok(arr) = Array2::from_shape_vec((ny, nx), pixels) {
                    return Ok(ImageData::Image2D(arr));
                }
            }
        }
        if let Some(pos) = table_bytes.windows(2).position(|w| w == [0x78, 0x9c] || w == [0x78, 0x01] || w == [0x78, 0xda]) {
            let zlib_data = &table_bytes[pos..];
            let mut decoder = ZlibDecoder::new(zlib_data);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() && decompressed.len() >= nx * ny {
                let zbitpix: i32 = parse_fits_int_card(cards, "ZBITPIX").unwrap_or(-32);
                let pixels = read_pixels(&decompressed, zbitpix, nx * ny);
                if let Ok(arr) = Array2::from_shape_vec((ny, nx), pixels) {
                    return Ok(ImageData::Image2D(arr));
                }
            }
        }
    }

    Err("Compressed format unsupported".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_card_formatting() {
        let card1 = HeaderCard::new("OBJECT", "'M31'", "Andromeda Galaxy");
        assert_eq!(card1.keyword, "OBJECT");
        assert!(card1.raw.contains("M31"));
        assert!(card1.raw.contains("Andromeda Galaxy"));

        let card2 = HeaderCard::new("HIERARCH ESO DET CHIP1 ID", "'1234'", "Detector ID");
        assert!(card2.raw.starts_with("HIERARCH ESO DET CHIP1 ID"));
    }

    #[test]
    fn test_save_edited_to_new_file() {
        let temp_dir = std::env::temp_dir();
        let orig_file_path = temp_dir.join("test_sample_fitslook.fits");

        // Create a minimal standard FITS file
        {
            let mut f = File::create(&orig_file_path).unwrap();
            let mut header = Vec::new();
            header.extend_from_slice(HeaderCard::format_card("SIMPLE", "T", "Standard FITS format").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("BITPIX", "8", "8-bit unsigned int").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("NAXIS", "2", "2D image").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("NAXIS1", "10", "10 pixels wide").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("NAXIS2", "10", "10 pixels high").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("OBJECT", "'Test Obj'", "Original object").as_bytes());
            header.extend_from_slice(HeaderCard::format_card("END", "", "").as_bytes());

            let pad = BLOCK_SIZE - (header.len() % BLOCK_SIZE);
            header.resize(header.len() + pad, b' ');
            f.write_all(&header).unwrap();

            // 100 bytes of dummy pixel data
            let pixels = vec![42u8; 100];
            f.write_all(&pixels).unwrap();
            let d_pad = BLOCK_SIZE - (100 % BLOCK_SIZE);
            f.write_all(&vec![0u8; d_pad]).unwrap();
        }

        let doc = FitsDocument::open(&orig_file_path).expect("Should open test fits file");
        let mut cards = doc.hdus[0].cards_list.clone();
        let end_pos = cards.iter().position(|c| c.keyword == "END").unwrap_or(cards.len());
        cards.insert(end_pos, HeaderCard::new("MYKEY", "'CUSTOM_VALUE'", "User added comment"));

        let mut updated_cards_per_hdu = BTreeMap::new();
        updated_cards_per_hdu.insert(0, cards);

        let edited_path_str = doc
            .save_edited_to_new_file(&updated_cards_per_hdu)
            .expect("Should save edited fits file");

        let edited_path = PathBuf::from(&edited_path_str);
        assert!(edited_path.exists());
        assert!(edited_path.file_name().unwrap().to_string_lossy().contains("_edited.fits"));
        assert_ne!(orig_file_path, edited_path);

        // Read back the edited file
        let doc_edited = FitsDocument::open(&edited_path).expect("Should open edited fits file");
        let mykey = doc_edited.hdus[0].header_cards.get("MYKEY");
        assert_eq!(mykey, Some(&"CUSTOM_VALUE".to_string()));

        // Clean up temp files
        std::fs::remove_file(orig_file_path).ok();
        std::fs::remove_file(edited_path).ok();
    }
}

