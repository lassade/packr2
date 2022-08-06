// original source copied from: texture_packer https://github.com/PistonDevelopers/texture_packer

use crate::Size;

use super::{Packer, PackerConfig, Rect, Rectf};
use std::cmp::max;

struct Skyline {
    pub x: u32,
    pub y: u32,
    pub w: u32,
}

impl Skyline {
    #[inline(always)]
    pub fn left(&self) -> u32 {
        self.x
    }

    #[inline(always)]
    pub fn right(&self) -> u32 {
        self.x + self.w - 1
    }
}

pub struct SkylinePacker {
    config: PackerConfig,
    // the skylines are sorted by their `x` position
    skylines: Vec<Skyline>,
    used_area: Size,
}

impl SkylinePacker {
    pub fn new(config: PackerConfig) -> Self {
        let skylines = vec![Skyline {
            x: 0,
            y: 0,
            w: config.max_width,
        }];

        SkylinePacker {
            config,
            skylines,
            used_area: Size::ZERO,
        }
    }

    // return `rect` if rectangle (w, h) can fit the skyline started at `i`
    fn can_put(&self, mut i: usize, w: u32, h: u32) -> Option<Rect> {
        let mut rect = Rect::new(self.skylines[i].x, 0, w, h);
        let mut width_left = rect.w;
        loop {
            rect.y = max(rect.y, self.skylines[i].y);
            // the source rect is too large
            if (rect.x + rect.w) > self.config.max_width
                || (rect.y + rect.h) > self.config.max_height
            {
                return None;
            }
            if self.skylines[i].w >= width_left {
                return Some(rect);
            }
            width_left -= self.skylines[i].w;
            i += 1;
            assert!(i < self.skylines.len());
        }
    }

    fn find_skyline(&self, w: u32, h: u32) -> Option<(usize, Rect)> {
        let mut bottom = std::u32::MAX;
        let mut width = std::u32::MAX;
        let mut index = None;
        let mut rect = Rect::new(0, 0, 0, 0);

        // keep the `bottom` and `width` as small as possible
        for i in 0..self.skylines.len() {
            if let Some(r) = self.can_put(i, w, h) {
                if r.bottom() < bottom || (r.bottom() == bottom && self.skylines[i].w < width) {
                    bottom = r.bottom();
                    width = self.skylines[i].w;
                    index = Some(i);
                    rect = r;
                }
            }

            if self.config.allow_flipping {
                if let Some(r) = self.can_put(i, h, w) {
                    if r.bottom() < bottom || (r.bottom() == bottom && self.skylines[i].w < width) {
                        bottom = r.bottom();
                        width = self.skylines[i].w;
                        index = Some(i);
                        rect = r;
                    }
                }
            }
        }

        index.map(|x| (x, rect))
    }

    fn split(&mut self, index: usize, rect: &Rect) {
        let skyline = Skyline {
            x: rect.left(),
            y: rect.bottom() + 1,
            w: rect.w,
        };

        assert!(skyline.right() <= self.config.max_width);
        assert!(skyline.y <= self.config.max_height);

        self.skylines.insert(index, skyline);

        let i = index + 1;
        while i < self.skylines.len() {
            assert!(self.skylines[i - 1].left() <= self.skylines[i].left());

            if self.skylines[i].left() <= self.skylines[i - 1].right() {
                let shrink = self.skylines[i - 1].right() - self.skylines[i].left() + 1;
                if self.skylines[i].w <= shrink {
                    self.skylines.remove(i);
                } else {
                    self.skylines[i].x += shrink;
                    self.skylines[i].w -= shrink;
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn merge(&mut self) {
        let mut i = 1;
        while i < self.skylines.len() {
            if self.skylines[i - 1].y == self.skylines[i].y {
                self.skylines[i - 1].w += self.skylines[i].w;
                self.skylines.remove(i);
                i -= 1;
            }
            i += 1;
        }
    }
}

impl Packer for SkylinePacker {
    fn insert(&mut self, w: u32, h: u32) -> Option<Rectf> {
        if let Some((i, rect)) = self.find_skyline(w, h) {
            self.split(i, &rect);
            self.merge();
            self.used_area.expand_with(&rect);
            Some(Rectf::from_rect(rect, w != rect.w))
        } else {
            None
        }
    }

    fn reset(&mut self, resize: Option<Size>) {
        if let Some(Size { w, h }) = resize {
            self.config.max_width = w;
            self.config.max_height = h;
        }
        self.used_area = Size::ZERO;
        self.skylines.clear();
        self.skylines.push(Skyline {
            x: 0,
            y: 0,
            w: self.config.max_width,
        });
    }

    fn used_area(&self) -> Size {
        self.used_area
    }
}
