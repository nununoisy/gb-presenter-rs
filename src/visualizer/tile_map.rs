use std::collections::HashMap;
use std::path::Path;
use image::{DynamicImage, Pixel, RgbaImage};
use raqote::{AntialiasMode, BlendMode, DrawOptions, DrawTarget, Image};
use image::io::Reader as ImageReader;

#[derive(Clone)]
pub struct TileMap {
    image: RgbaImage,
    char_map: String,
    rows: usize,
    cols: usize,
    tile_w: usize,
    tile_h: usize,
    tile_cache: HashMap<char, Vec<u32>>
}

impl TileMap {
    pub fn new(image_data: &[u8], tile_w: usize, tile_h: usize, char_map: &str) -> Result<Self, String> {
        let image = image::load_from_memory(image_data)
            .map_err(|e| e.to_string())?;
        Self::from_image(image, tile_w, tile_h, char_map)
    }

    pub fn open<P: AsRef<Path>>(image_path: P, tile_w: usize, tile_h: usize, char_map: &str) -> Result<Self, String> {
        let image = ImageReader::open(image_path)
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?;
        Self::from_image(image, tile_w, tile_h, char_map)
    }

    pub fn from_image(image: DynamicImage, tile_w: usize, tile_h: usize, char_map: &str) -> Result<Self, String> {
        if image.width() as usize % tile_w != 0 {
            return Err("Image width is not a multiple of tile width".to_string());
        }
        if image.height() as usize % tile_h != 0 {
            return Err("Image height is not a multiple of tile height".to_string());
        }

        let cols = image.width() as usize / tile_w;
        let rows = image.height() as usize / tile_h;

        Ok(Self {
            image: image.to_rgba8(),
            char_map: char_map.to_string(),
            rows,
            cols,
            tile_w,
            tile_h,
            tile_cache: HashMap::new()
        })
    }

    pub fn tile_w(&self) -> usize {
        self.tile_w
    }

    pub fn tile_h(&self) -> usize {
        self.tile_h
    }

    pub fn tile_buf(&mut self, c: char) -> Option<Vec<u32>> {
        Some(match self.tile_cache.get(&c) {
            Some(cached_buf) => cached_buf.clone(),
            None => {
                let tile_index = self.char_map.find(c)?;
                let col = tile_index % self.cols;
                let row = tile_index / self.cols;

                let tile_buf: Vec<u32> = (0..self.tile_h*self.tile_w)
                    .map(|i| {
                        let dx = i % self.tile_w;
                        let dy = i / self.tile_w;
                        let x = col * self.tile_w + dx;
                        let y = row * self.tile_h + dy;

                        let p = self.image.get_pixel(x as _, y as _).channels();
                        // convert to ARGB
                        ((p[3] as u32) << 24) | ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32)
                    })
                    .collect();
                self.tile_cache.insert(c, tile_buf.clone());
                tile_buf.clone()
            }
        })
    }

    pub fn draw_tile(&mut self, dt: &mut DrawTarget, c: char, x: f32, y: f32, alpha: f32) {
        let tile_buf = self.tile_buf(c);
        if tile_buf.is_none() {
            return;
        }

        let image = Image {
            width: self.tile_w as _,
            height: self.tile_h as _,
            data: &tile_buf.unwrap(),
        };

        dt.draw_image_at(x, y, &image, &DrawOptions {
            blend_mode: BlendMode::SrcOver,
            alpha,
            antialias: AntialiasMode::None,
        });
    }

    pub fn draw_text(&mut self, dt: &mut DrawTarget, text: &str, x: f32, y: f32, alpha: f32) {
        for (i, c) in text.chars().enumerate() {
            let dx = (i as f32) * (self.tile_w as f32);
            self.draw_tile(dt, c, x + dx, y, alpha);
        }
    }
}
