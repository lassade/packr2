// useful links:
// https://cgi.csc.liv.ac.uk/~epa/surveyhtml.html
// https://github.com/emilk/egui look for texture_atlas.rs

use crate::{Packer, PackerConfig, Rectf};

/// Same implementation used by `egui`.
#[derive(Clone)]
pub struct StripPacker {
    config: PackerConfig,
    /// Used for when allocating new rectangles.
    cursor: [u32; 2],
    row_height: u32,
    /// Set when someone requested more space than was available.
    overflowed: bool,
}

impl StripPacker {
    pub fn new(config: PackerConfig) -> Self {
        Self {
            config,
            cursor: [0; 2],
            row_height: 0,
            overflowed: false,
        }
    }

    pub const fn cursor(&self) -> [u32; 2] {
        self.cursor
    }

    /// When this get high, it might be time to clear and start over!
    pub fn fill_ratio(&self) -> f32 {
        if self.overflowed {
            1.0
        } else {
            (self.cursor[1] + self.row_height) as f32 / self.config.max_height as f32
        }
    }
}

impl Packer for StripPacker {
    fn insert(&mut self, w: u32, h: u32) -> Option<Rectf> {
        // this current algorithm works best for fonts
        // because they all use the have about the same height

        // todo: keep previous rows available until there's some space left
        // todo: hability to rotate images and better fit other images

        if w > self.config.max_width {
            return None;
        }

        if self.cursor[0] + w > self.config.max_width {
            // new row:
            self.cursor[0] = 0;
            self.cursor[1] += self.row_height;
            self.row_height = 0;
        }

        self.row_height = self.row_height.max(h);
        let required_height = self.cursor[1] + self.row_height;

        if required_height > self.config.max_height {
            self.overflowed = true;
            return None;
        }

        let pos = self.cursor;
        self.cursor[0] += w;

        Some(Rectf {
            x: pos[0],
            y: pos[1],
            w,
            h,
            flipped: false,
        })
    }

    fn reset(&mut self) {
        self.cursor = [0; 2];
        self.row_height = 0;
        self.overflowed = false;
    }
}
