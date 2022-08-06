//! Rectangle packing algorithm

// useful refs https://cgi.csc.liv.ac.uk/~epa/surveyhtml.html
// todo: include egui algorithm witch is a strip_packer
// todo: include maybe `rectpack2D` (but using a dynamic version)
// todo: https://blackpawn.com/texts/lightmaps/default.html

// original source copied from: texture_packer https://github.com/PistonDevelopers/texture_packer

use std::ops::Deref;

pub use skyline_packer::SkylinePacker;
pub use split_packer::SplitPacker;
pub use strip_packer::StripPacker;

//mod optimize;
mod skyline_packer;
mod split_packer;
mod strip_packer;

/// Configuration for a texture packer.
#[derive(Debug, Copy, Clone)]
pub struct PackerConfig {
    /// Max width of the packed image. Default value is `1024`.
    pub max_width: u32,
    /// Max height of the packed image. Default value is `1024`.
    pub max_height: u32,
    /// True to allow rotation of the input images. Default value is `true`. Images rotated will be
    /// rotated 90 degrees clockwise.
    pub allow_flipping: bool,
}

impl Default for PackerConfig {
    fn default() -> PackerConfig {
        PackerConfig {
            max_width: 1024,
            max_height: 1024,
            allow_flipping: true,
        }
    }
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Rect {
    /// Create a new [Rect] based on a position and its width and height.
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    /// Get the top coordinate of the rectangle.
    #[inline(always)]
    pub fn top(&self) -> u32 {
        self.y
    }

    /// Get the bottom coordinate of the rectangle.
    #[inline(always)]
    pub fn bottom(&self) -> u32 {
        // todo: badly defined, remove the -1
        self.y + self.h - 1
    }

    /// Get the left coordinate of the rectangle.
    #[inline(always)]
    pub fn left(&self) -> u32 {
        self.x
    }

    /// Get the right coordinate of the rectangle.
    #[inline(always)]
    pub fn right(&self) -> u32 {
        // todo: badly defined, remove the -1
        self.x + self.w - 1
    }

    /// Check if this rectangle contains another.
    pub fn contains(&self, other: &Rect) -> bool {
        self.left() <= other.left()
            && self.right() >= other.right()
            && self.top() <= other.top()
            && self.bottom() >= other.bottom()
    }
}

/// [`Rect`] that could be flipped sideway (rotated by 90 degrees clockwise)
#[repr(C)]
pub struct Rectf {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub flipped: bool,
}

impl Rectf {
    pub fn from_rect(Rect { x, y, w, h }: Rect, flipped: bool) -> Self {
        Self {
            x,
            y,
            w,
            h,
            flipped,
        }
    }
}

impl Deref for Rectf {
    type Target = Rect;

    fn deref(&self) -> &Self::Target {
        // safety: the fields of `Rect` are included inside `Rectf` in the same order
        unsafe { core::mem::transmute(self) }
    }
}

#[derive(Default, Clone, Copy)]
pub struct Size {
    pub w: u32,
    pub h: u32,
}

impl Size {
    pub const fn new(w: u32, h: u32) -> Self {
        Self { w, h }
    }

    pub fn flip(&mut self) -> &Self {
        core::mem::swap(&mut self.w, &mut self.h);
        self
    }

    pub const fn max_side(&self) -> u32 {
        if self.h > self.w {
            self.h
        } else {
            self.w
        }
    }

    pub const fn min_side(&self) -> u32 {
        if self.h < self.w {
            self.h
        } else {
            self.w
        }
    }

    pub const fn area(&self) -> u32 {
        self.w * self.h
    }

    pub const fn perimeter(&self) -> u32 {
        2 * self.w + 2 * self.h
    }

    pub fn pathological_mult(&self) -> f32 {
        self.max_side() as f32 / self.min_side() as f32 * self.area() as f32
    }

    pub fn expand_with(&mut self, r: &Rect) {
        self.w = self.w.max(r.x + r.w);
        self.h = self.h.max(r.y + r.h);
    }
}

pub trait Packer {
    fn insert(&mut self, w: u32, h: u32) -> Option<Rectf>;
    fn reset(&mut self);
}
