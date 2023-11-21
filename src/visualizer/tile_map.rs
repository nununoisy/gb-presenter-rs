use tiny_skia::{BlendMode, FilterQuality, IntRect, Pixmap, PixmapMut, PixmapPaint, PixmapRef, Point, Transform};

#[derive(Clone)]
pub struct TileMap {
    image: Pixmap,
    char_map: String,
    rows: usize,
    cols: usize,
    tile_w: usize,
    tile_h: usize,
    tile_cache: Vec<Option<Pixmap>>
}

impl TileMap {
    pub fn new(image: Pixmap, tile_w: usize, tile_h: usize, char_map: &str) -> Self {
        let rows = image.height() as usize / tile_h;
        let cols = image.width() as usize / tile_w;

        Self {
            image,
            char_map: char_map.to_string(),
            rows,
            cols,
            tile_w,
            tile_h,
            tile_cache: vec![None; char_map.chars().count()]
        }
    }

    pub fn tile_w(&self) -> usize {
        self.tile_w
    }

    pub fn tile_h(&self) -> usize {
        self.tile_h
    }

    pub fn tile_pixmap(&mut self, c: char) -> Option<PixmapRef> {
        let tile_index = self.char_map.chars().position(|mc| mc == c)?;

        if self.tile_cache.get(tile_index)?.is_none() {
            let col = tile_index % self.cols;
            let row = tile_index / self.cols;
            debug_assert!(row < self.rows);

            let tile_rect = IntRect::from_xywh(
                (col * self.tile_w) as i32,
                (row * self.tile_h) as i32,
                self.tile_w as u32,
                self.tile_h as u32
            )?;
            self.tile_cache[tile_index] = self.image.clone_rect(tile_rect);
        }

        self.tile_cache[tile_index].as_ref().map(|t| t.as_ref())
    }

    pub fn draw_tile(&mut self, dt: &mut PixmapMut<'_>, c: char, pos: Point, opacity: f32) {
        if let Some(tile_pixmap) = self.tile_pixmap(c) {
            dt.draw_pixmap(
                pos.x as i32,
                pos.y as i32,
                tile_pixmap,
                &PixmapPaint {
                    opacity,
                    blend_mode: BlendMode::SourceOver,
                    quality: FilterQuality::Nearest
                },
                Transform::identity(),
                None
            )
        }
    }

    pub fn draw_text(&mut self, dt: &mut PixmapMut<'_>, text: &str, pos: Point, opacity: f32) {
        for (i, c) in text.chars().enumerate() {
            let dx = (i * self.tile_w) as f32;
            self.draw_tile(dt, c, pos + Point::from_xy(dx, 0.0), opacity);
        }
    }
}
