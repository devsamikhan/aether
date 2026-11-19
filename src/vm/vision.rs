// ==============================================================================
// AetherVision — Pure-Rust Computer Vision & Image Processing Engine
// Zero OpenCV / Zero Pillow / Zero External Crates
// Supports BMP & PPM/PGM I/O, Bilinear Resize, Grayscale, Crop, Flip,
// Sobel Edge Detection & Direct NCHW Tensor Conversion for CNNs
// ==============================================================================

use super::tensor::{get_tensor_data, wrap_tensor_handle, TensorData};
use super::value::Value;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Debug)]
pub struct ImageData {
    pub width: usize,
    pub height: usize,
    pub channels: usize,  // 1 (Gray), 3 (RGB), 4 (RGBA)
    pub pixels: Vec<f32>, // HWC layout, normalized [0.0, 1.0] (or arbitrary after normalize)
}

impl ImageData {
    pub fn new(
        width: usize,
        height: usize,
        channels: usize,
        pixels: Vec<f32>,
    ) -> Result<Self, String> {
        if width == 0 || height == 0 || !matches!(channels, 1 | 3 | 4) {
            return Err(format!(
                "Invalid image dims ({}x{}x{}); channels must be 1, 3, or 4",
                width, height, channels
            ));
        }
        let expected = width * height * channels;
        if pixels.len() != expected {
            return Err(format!(
                "Image pixel count mismatch: expected {}, got {}",
                expected,
                pixels.len()
            ));
        }
        Ok(Self {
            width,
            height,
            channels,
            pixels,
        })
    }

    #[inline]
    fn idx(&self, x: usize, y: usize, c: usize) -> usize {
        (y * self.width + x) * self.channels + c
    }

    pub fn resize_bilinear(&self, new_w: usize, new_h: usize) -> Result<Self, String> {
        if new_w == 0 || new_h == 0 {
            return Err("Target resize dimensions must be positive".into());
        }
        let mut out = vec![0.0f32; new_w * new_h * self.channels];
        let x_ratio = if new_w > 1 {
            (self.width - 1) as f32 / (new_w - 1) as f32
        } else {
            0.0
        };
        let y_ratio = if new_h > 1 {
            (self.height - 1) as f32 / (new_h - 1) as f32
        } else {
            0.0
        };

        for ny in 0..new_h {
            let fy = ny as f32 * y_ratio;
            let y0 = (fy.floor() as usize).min(self.height - 1);
            let y1 = (y0 + 1).min(self.height - 1);
            let dy = fy - y0 as f32;

            for nx in 0..new_w {
                let fx = nx as f32 * x_ratio;
                let x0 = (fx.floor() as usize).min(self.width - 1);
                let x1 = (x0 + 1).min(self.width - 1);
                let dx = fx - x0 as f32;

                for c in 0..self.channels {
                    let v00 = self.pixels[self.idx(x0, y0, c)];
                    let v10 = self.pixels[self.idx(x1, y0, c)];
                    let v01 = self.pixels[self.idx(x0, y1, c)];
                    let v11 = self.pixels[self.idx(x1, y1, c)];
                    let top = v00 * (1.0 - dx) + v10 * dx;
                    let bot = v01 * (1.0 - dx) + v11 * dx;
                    out[(ny * new_w + nx) * self.channels + c] = top * (1.0 - dy) + bot * dy;
                }
            }
        }
        Self::new(new_w, new_h, self.channels, out)
    }

    pub fn to_grayscale(&self) -> Self {
        if self.channels == 1 {
            return self.clone();
        }
        let mut out = vec![0.0f32; self.width * self.height];
        for y in 0..self.height {
            for x in 0..self.width {
                let r = self.pixels[self.idx(x, y, 0)];
                let g = self.pixels[self.idx(x, y, 1)];
                let b = self.pixels[self.idx(x, y, 2)];
                out[y * self.width + x] = 0.299 * r + 0.587 * g + 0.114 * b;
            }
        }
        Self {
            width: self.width,
            height: self.height,
            channels: 1,
            pixels: out,
        }
    }

    pub fn crop(&self, x0: usize, y0: usize, w: usize, h: usize) -> Result<Self, String> {
        if w == 0 || h == 0 || x0 + w > self.width || y0 + h > self.height {
            return Err(format!(
                "Crop region ({},{},{}x{}) out of bounds for {}x{}",
                x0, y0, w, h, self.width, self.height
            ));
        }
        let mut out = vec![0.0f32; w * h * self.channels];
        for y in 0..h {
            for x in 0..w {
                for c in 0..self.channels {
                    out[(y * w + x) * self.channels + c] = self.pixels[self.idx(x0 + x, y0 + y, c)];
                }
            }
        }
        Self::new(w, h, self.channels, out)
    }

    pub fn flip_horizontal(&self) -> Self {
        let mut out = vec![0.0f32; self.pixels.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let sx = self.width - 1 - x;
                for c in 0..self.channels {
                    out[self.idx(x, y, c)] = self.pixels[self.idx(sx, y, c)];
                }
            }
        }
        Self {
            width: self.width,
            height: self.height,
            channels: self.channels,
            pixels: out,
        }
    }

    pub fn normalize(&self, mean: f32, std: f32) -> Self {
        let inv_std = 1.0 / std.max(1e-6);
        let pixels = self.pixels.iter().map(|&p| (p - mean) * inv_std).collect();
        Self {
            width: self.width,
            height: self.height,
            channels: self.channels,
            pixels,
        }
    }

    /// Computes 3x3 Sobel gradient magnitude edge map (returns 1-channel image).
    pub fn sobel_edges(&self) -> Self {
        let gray = self.to_grayscale();
        let w = gray.width;
        let h = gray.height;
        let mut out = vec![0.0f32; w * h];
        if w < 3 || h < 3 {
            return Self {
                width: w,
                height: h,
                channels: 1,
                pixels: out,
            };
        }
        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let p = |xx: usize, yy: usize| gray.pixels[yy * w + xx];
                let gx = -p(x - 1, y - 1) + p(x + 1, y - 1) - 2.0 * p(x - 1, y) + 2.0 * p(x + 1, y)
                    - p(x - 1, y + 1)
                    + p(x + 1, y + 1);
                let gy = -p(x - 1, y - 1) - 2.0 * p(x, y - 1) - p(x + 1, y - 1)
                    + p(x - 1, y + 1)
                    + 2.0 * p(x, y + 1)
                    + p(x + 1, y + 1);
                out[y * w + x] = (gx * gx + gy * gy).sqrt().min(1.0);
            }
        }
        Self {
            width: w,
            height: h,
            channels: 1,
            pixels: out,
        }
    }

    /// Converts HWC image into NCHW `[1, C, H, W]` TensorData for `NN.conv2d`.
    pub fn to_nchw_tensor(&self) -> Result<TensorData, String> {
        let mut data = vec![0.0f64; self.channels * self.height * self.width];
        let plane = self.height * self.width;
        for y in 0..self.height {
            for x in 0..self.width {
                for c in 0..self.channels {
                    data[c * plane + y * self.width + x] = self.pixels[self.idx(x, y, c)] as f64;
                }
            }
        }
        TensorData::new(vec![1, self.channels, self.height, self.width], data)
    }

    pub fn from_tensor(td: &TensorData) -> Result<Self, String> {
        let (c, h, w) = match td.shape.as_slice() {
            [h, w] => (1, *h, *w),
            [c, h, w] => (*c, *h, *w),
            [1, c, h, w] => (*c, *h, *w),
            other => {
                return Err(format!(
                    "Vision.from_tensor expects [H,W], [C,H,W], or [1,C,H,W], got {:?}",
                    other
                ))
            }
        };
        let mut pixels = vec![0.0f32; h * w * c];
        let plane = h * w;
        for ch in 0..c {
            for y in 0..h {
                for x in 0..w {
                    pixels[(y * w + x) * c + ch] = td.data[ch * plane + y * w + x] as f32;
                }
            }
        }
        Self::new(w, h, c, pixels)
    }

    /// Encodes image as 24-bit uncompressed Windows BMP.
    pub fn encode_bmp(&self) -> Vec<u8> {
        let row_bytes = self.width * 3;
        let row_stride = row_bytes.div_ceil(4) * 4;
        let pixel_bytes = row_stride * self.height;
        let file_size = 54 + pixel_bytes;

        let mut out = Vec::with_capacity(file_size);
        // BITMAPFILEHEADER (14 bytes)
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&54u32.to_le_bytes());
        // BITMAPINFOHEADER (40 bytes)
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(self.width as i32).to_le_bytes());
        out.extend_from_slice(&(self.height as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&24u16.to_le_bytes()); // bpp
        out.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
        out.extend_from_slice(&(pixel_bytes as u32).to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&2835i32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());

        let pad = row_stride - row_bytes;
        for y in (0..self.height).rev() {
            for x in 0..self.width {
                let (r, g, b) = if self.channels == 1 {
                    let v = self.pixels[self.idx(x, y, 0)];
                    (v, v, v)
                } else {
                    (
                        self.pixels[self.idx(x, y, 0)],
                        self.pixels[self.idx(x, y, 1)],
                        self.pixels[self.idx(x, y, 2)],
                    )
                };
                out.push((b.clamp(0.0, 1.0) * 255.0).round() as u8);
                out.push((g.clamp(0.0, 1.0) * 255.0).round() as u8);
                out.push((r.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
            out.extend(std::iter::repeat_n(0u8, pad));
        }
        out
    }

    pub fn decode_bmp(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 54 || &bytes[0..2] != b"BM" {
            return Err("Invalid BMP header".into());
        }
        let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
        let w_i = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
        let h_i = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
        let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);
        if w_i <= 0 || h_i == 0 || (bpp != 24 && bpp != 32) {
            return Err(format!(
                "Unsupported BMP format: {}x{} @ {}bpp (supported: 24/32bpp)",
                w_i, h_i, bpp
            ));
        }
        let width = w_i as usize;
        let height = h_i.unsigned_abs() as usize;
        let top_down = h_i < 0;
        let src_ch = (bpp / 8) as usize;
        let row_stride = (width * src_ch).div_ceil(4) * 4;
        if data_offset + row_stride * height > bytes.len() {
            return Err("Truncated BMP pixel payload".into());
        }
        let mut pixels = vec![0.0f32; width * height * 3];
        for row in 0..height {
            let y = if top_down { row } else { height - 1 - row };
            let src_row = &bytes[data_offset + row * row_stride..];
            for x in 0..width {
                let b = src_row[x * src_ch] as f32 / 255.0;
                let g = src_row[x * src_ch + 1] as f32 / 255.0;
                let r = src_row[x * src_ch + 2] as f32 / 255.0;
                let dst = (y * width + x) * 3;
                pixels[dst] = r;
                pixels[dst + 1] = g;
                pixels[dst + 2] = b;
            }
        }
        Self::new(width, height, 3, pixels)
    }

    pub fn encode_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        for y in 0..self.height {
            for x in 0..self.width {
                let (r, g, b) = if self.channels == 1 {
                    let v = self.pixels[self.idx(x, y, 0)];
                    (v, v, v)
                } else {
                    (
                        self.pixels[self.idx(x, y, 0)],
                        self.pixels[self.idx(x, y, 1)],
                        self.pixels[self.idx(x, y, 2)],
                    )
                };
                out.push((r.clamp(0.0, 1.0) * 255.0).round() as u8);
                out.push((g.clamp(0.0, 1.0) * 255.0).round() as u8);
                out.push((b.clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        out
    }

    pub fn decode_pnm(bytes: &[u8]) -> Result<Self, String> {
        let mut i = 0usize;
        let next_tok = |pos: &mut usize| -> Option<String> {
            while *pos < bytes.len() {
                if bytes[*pos] == b'#' {
                    while *pos < bytes.len() && bytes[*pos] != b'\n' {
                        *pos += 1;
                    }
                } else if bytes[*pos].is_ascii_whitespace() {
                    *pos += 1;
                } else {
                    break;
                }
            }
            let start = *pos;
            while *pos < bytes.len() && !bytes[*pos].is_ascii_whitespace() {
                *pos += 1;
            }
            if *pos > start {
                Some(String::from_utf8_lossy(&bytes[start..*pos]).into_owned())
            } else {
                None
            }
        };

        let magic = next_tok(&mut i).ok_or_else(|| "Empty PNM file".to_string())?;
        let width: usize = next_tok(&mut i)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| "Invalid PNM width".to_string())?;
        let height: usize = next_tok(&mut i)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| "Invalid PNM height".to_string())?;
        let max_val: f32 = next_tok(&mut i)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| "Invalid PNM maxval".to_string())?;
        if i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let channels = match magic.as_str() {
            "P6" => 3,
            "P5" => 1,
            other => return Err(format!("Unsupported PNM magic '{}' (use P5/P6)", other)),
        };
        let expected = width * height * channels;
        if i + expected > bytes.len() {
            return Err("Truncated PNM pixel data".into());
        }
        let inv = 1.0 / max_val.max(1.0);
        let pixels: Vec<f32> = bytes[i..i + expected]
            .iter()
            .map(|&b| (b as f32) * inv)
            .collect();
        Self::new(width, height, channels, pixels)
    }

    pub fn load_file(path: &str) -> Result<Self, String> {
        let bytes =
            fs::read(path).map_err(|e| format!("Failed to read image '{}': {}", path, e))?;
        if bytes.starts_with(b"BM") {
            Self::decode_bmp(&bytes)
        } else if bytes.starts_with(b"P5") || bytes.starts_with(b"P6") {
            Self::decode_pnm(&bytes)
        } else {
            Err(format!(
                "Unsupported image format in '{}' (supported: BMP, PPM P6, PGM P5)",
                path
            ))
        }
    }
}

static NEXT_IMG_ID: AtomicU64 = AtomicU64::new(1);
static IMAGES: OnceLock<Mutex<HashMap<u64, Arc<ImageData>>>> = OnceLock::new();

fn images() -> &'static Mutex<HashMap<u64, Arc<ImageData>>> {
    IMAGES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn store_img(img: ImageData) -> u64 {
    let id = NEXT_IMG_ID.fetch_add(1, Ordering::SeqCst);
    images().lock().insert(id, Arc::new(img));
    id
}

fn get_img(val: &Value) -> Result<Arc<ImageData>, String> {
    let id = match val {
        Value::Int(i) if *i > 0 => *i as u64,
        _ => return Err("Expected positive integer image handle".into()),
    };
    images()
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("Image handle {} not found", id))
}

fn arg_usize(val: &Value, name: &str) -> Result<usize, String> {
    match val {
        Value::Int(i) if *i >= 0 => Ok(*i as usize),
        _ => Err(format!("Expected non-negative integer for {}", name)),
    }
}

fn arg_f32(val: &Value, name: &str) -> Result<f32, String> {
    match val {
        Value::Float(f) => Ok(*f as f32),
        Value::Int(i) => Ok(*i as f32),
        _ => Err(format!("Expected number for {}", name)),
    }
}

pub fn register_vision_module(globals: &mut HashMap<String, Value>) {
    let mut vmod = HashMap::new();

    vmod.insert(
        "create".to_string(),
        Value::Native("Vision.create".into(), |args| {
            if args.len() < 3 {
                return Err(
                    "Vision.create(width, height, channels, fill_or_pixels?) requires 3 args"
                        .into(),
                );
            }
            let w = arg_usize(&args[0], "width")?;
            let h = arg_usize(&args[1], "height")?;
            let c = arg_usize(&args[2], "channels")?;
            let len = w * h * c;
            let pixels = match args.get(3) {
                Some(Value::Array(arr)) => arr
                    .lock()
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => *f as f32,
                        Value::Int(i) => *i as f32,
                        _ => 0.0,
                    })
                    .collect(),
                Some(Value::Float(f)) => vec![*f as f32; len],
                Some(Value::Int(i)) => vec![*i as f32; len],
                _ => vec![0.0f32; len],
            };
            let img = ImageData::new(w, h, c, pixels)?;
            Ok(Value::Int(store_img(img) as i64))
        }),
    );

    vmod.insert(
        "load".to_string(),
        Value::Native("Vision.load".into(), |args| {
            if args.is_empty() {
                return Err("Vision.load(path) requires image path".into());
            }
            let path = format!("{}", args[0]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_read(&path))?;
            let img = ImageData::load_file(&path)?;
            Ok(Value::Int(store_img(img) as i64))
        }),
    );

    vmod.insert(
        "save_bmp".to_string(),
        Value::Native("Vision.save_bmp".into(), |args| {
            if args.len() < 2 {
                return Err("Vision.save_bmp(img_id, path) requires 2 arguments".into());
            }
            let img = get_img(&args[0])?;
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;
            fs::write(&path, img.encode_bmp())
                .map_err(|e| format!("Failed to save BMP '{}': {}", path, e))?;
            Ok(Value::Bool(true))
        }),
    );

    vmod.insert(
        "save_ppm".to_string(),
        Value::Native("Vision.save_ppm".into(), |args| {
            if args.len() < 2 {
                return Err("Vision.save_ppm(img_id, path) requires 2 arguments".into());
            }
            let img = get_img(&args[0])?;
            let path = format!("{}", args[1]);
            crate::vm::interpreter::with_current_vm(|vm| vm.security_policy.check_fs_write(&path))?;
            fs::write(&path, img.encode_ppm())
                .map_err(|e| format!("Failed to save PPM '{}': {}", path, e))?;
            Ok(Value::Bool(true))
        }),
    );

    vmod.insert(
        "info".to_string(),
        Value::Native("Vision.info".into(), |args| {
            if args.is_empty() {
                return Err("Vision.info(img_id) requires img_id".into());
            }
            let img = get_img(&args[0])?;
            let min_v = img.pixels.iter().copied().fold(f32::INFINITY, f32::min);
            let max_v = img.pixels.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let mean_v = img.pixels.iter().sum::<f32>() / (img.pixels.len().max(1) as f32);
            let mut m = HashMap::new();
            m.insert("width".to_string(), Value::Int(img.width as i64));
            m.insert("height".to_string(), Value::Int(img.height as i64));
            m.insert("channels".to_string(), Value::Int(img.channels as i64));
            m.insert("min".to_string(), Value::Float(min_v as f64));
            m.insert("max".to_string(), Value::Float(max_v as f64));
            m.insert("mean".to_string(), Value::Float(mean_v as f64));
            Ok(Value::map(m))
        }),
    );

    vmod.insert(
        "resize".to_string(),
        Value::Native("Vision.resize".into(), |args| {
            if args.len() < 3 {
                return Err("Vision.resize(img_id, new_w, new_h) requires 3 arguments".into());
            }
            let img = get_img(&args[0])?;
            let nw = arg_usize(&args[1], "new_w")?;
            let nh = arg_usize(&args[2], "new_h")?;
            let out = img.resize_bilinear(nw, nh)?;
            Ok(Value::Int(store_img(out) as i64))
        }),
    );

    vmod.insert(
        "grayscale".to_string(),
        Value::Native("Vision.grayscale".into(), |args| {
            if args.is_empty() {
                return Err("Vision.grayscale(img_id) requires img_id".into());
            }
            let img = get_img(&args[0])?;
            Ok(Value::Int(store_img(img.to_grayscale()) as i64))
        }),
    );

    vmod.insert(
        "crop".to_string(),
        Value::Native("Vision.crop".into(), |args| {
            if args.len() < 5 {
                return Err("Vision.crop(img_id, x, y, w, h) requires 5 arguments".into());
            }
            let img = get_img(&args[0])?;
            let x = arg_usize(&args[1], "x")?;
            let y = arg_usize(&args[2], "y")?;
            let w = arg_usize(&args[3], "w")?;
            let h = arg_usize(&args[4], "h")?;
            let out = img.crop(x, y, w, h)?;
            Ok(Value::Int(store_img(out) as i64))
        }),
    );

    vmod.insert(
        "flip_h".to_string(),
        Value::Native("Vision.flip_h".into(), |args| {
            if args.is_empty() {
                return Err("Vision.flip_h(img_id) requires img_id".into());
            }
            let img = get_img(&args[0])?;
            Ok(Value::Int(store_img(img.flip_horizontal()) as i64))
        }),
    );

    vmod.insert(
        "normalize".to_string(),
        Value::Native("Vision.normalize".into(), |args| {
            if args.len() < 3 {
                return Err("Vision.normalize(img_id, mean, std) requires 3 arguments".into());
            }
            let img = get_img(&args[0])?;
            let mean = arg_f32(&args[1], "mean")?;
            let std = arg_f32(&args[2], "std")?;
            Ok(Value::Int(store_img(img.normalize(mean, std)) as i64))
        }),
    );

    vmod.insert(
        "sobel_edges".to_string(),
        Value::Native("Vision.sobel_edges".into(), |args| {
            if args.is_empty() {
                return Err("Vision.sobel_edges(img_id) requires img_id".into());
            }
            let img = get_img(&args[0])?;
            Ok(Value::Int(store_img(img.sobel_edges()) as i64))
        }),
    );

    vmod.insert(
        "to_tensor".to_string(),
        Value::Native("Vision.to_tensor".into(), |args| {
            if args.is_empty() {
                return Err("Vision.to_tensor(img_id) requires img_id".into());
            }
            let img = get_img(&args[0])?;
            let td = img.to_nchw_tensor()?;
            Ok(wrap_tensor_handle(td))
        }),
    );

    vmod.insert(
        "from_tensor".to_string(),
        Value::Native("Vision.from_tensor".into(), |args| {
            if args.is_empty() {
                return Err("Vision.from_tensor(tensor) requires tensor".into());
            }
            let td = get_tensor_data(&args[0])?;
            let img = ImageData::from_tensor(&td)?;
            Ok(Value::Int(store_img(img) as i64))
        }),
    );

    globals.insert("Vision".to_string(), Value::map(vmod.clone()));
    globals.insert("__native_vision".to_string(), Value::map(vmod));
}
