//! Rectangle packing algorithm

#![no_std]

extern crate alloc;

use alloc::{vec, vec::Vec};
use core::cmp::Ordering;

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
#[derive(Default, Copy, Clone, Debug)]
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

    pub const fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }

    pub const fn size(&self) -> Size {
        Size {
            w: self.w,
            h: self.h,
        }
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
#[derive(Default, Clone, Copy, Debug)]
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

impl core::ops::Deref for Rectf {
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
    pub const ZERO: Size = Size::new(0, 0);

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

    pub const fn area(&self) -> u64 {
        self.w as u64 * self.h as u64
    }

    pub const fn perimeter(&self) -> u64 {
        2 * (self.w as u64) + 2 * (self.h as u64)
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
    fn reset(&mut self, resize: Option<Size>);
    fn used_area(&self) -> Size;
}

#[derive(Clone, Copy)]
pub struct RectInput<K> {
    pub size: Size,
    pub key: K,
}

#[derive(Clone, Copy)]
pub struct RectOutput<K> {
    pub rect: Rectf,
    pub atlas: usize,
    pub key: K,
}

pub const RECT_SORT_FUNCTIONS: [fn(Size, Size) -> Ordering; 6] = [
    |a: Size, b: Size| b.area().cmp(&a.area()),
    |a: Size, b: Size| b.perimeter().cmp(&a.perimeter()),
    |a: Size, b: Size| (b.w.max(b.h)).cmp(&(a.w.max(a.h))),
    |a: Size, b: Size| b.w.cmp(&a.w),
    |a: Size, b: Size| b.h.cmp(&a.h),
    |a: Size, b: Size| {
        if b.pathological_mult() < a.pathological_mult() {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    },
];

/// Sorts the input data using the heuristics defined in [`RECT_SORT_FUNCTIONS`] to find the best possible packing,
/// the results might end up been inside multiple atlases.
///
/// The output is sorted by atlas.
pub fn pack<P: Packer, K: Copy>(
    inputs: &mut Vec<RectInput<K>>,
    mut packer: P,
) -> Vec<RectOutput<K>> {
    let mut output = vec![];
    let mut output_area = core::u64::MAX;

    let mut current = vec![];
    let mut current_area;

    for cmp in RECT_SORT_FUNCTIONS {
        current.clear();
        current_area = 0;

        inputs.sort_by(|a, b| (cmp)(a.size, b.size));
        let mut iterator = inputs.iter();

        // use as many atlas as needed
        let mut atlas = 0;
        'atlasing: loop {
            packer.reset(None);

            loop {
                if let Some(input) = iterator.next() {
                    if let Some(rect) = packer.insert(input.size.w, input.size.h) {
                        current.push(RectOutput {
                            rect,
                            atlas,
                            key: input.key,
                        });
                    } else {
                        // use another atlas
                        current_area += packer.used_area().area();
                        atlas += 1;
                        break;
                    }
                } else {
                    break 'atlasing;
                }
            }
        }

        current_area += packer.used_area().area();

        if current_area < output_area {
            output_area = current_area;
            core::mem::swap(&mut current, &mut output);
        }
    }

    output
}
