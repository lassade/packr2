//! Rectangle packing algorithm

// useful refs https://cgi.csc.liv.ac.uk/~epa/surveyhtml.html
// todo: include egui algorithm witch is a strip_packer
// todo: include maybe `rectpack2D` (but using a dynamic version)
// todo: https://blackpawn.com/texts/lightmaps/default.html

// original source copied from: texture_packer https://github.com/PistonDevelopers/texture_packer

pub use skyline_packer::SkylinePacker;
pub use strip_packer::StripPacker;

mod rectpack2d_packer;
mod skyline_packer;
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
    pub allow_rotation: bool,
    /// Size of the padding between frames in pixel. Default value is `2`
    ///
    /// On some low-precision GPUs characters get muddled up
    /// if we don't add some empty pixels between the characters.
    /// On modern high-precision GPUs this is not needed.
    pub texture_padding: u32,
    /// Size of the repeated pixels at the border of each image. Default value is `0`.
    pub texture_extrusion: u32,
}

impl Default for PackerConfig {
    fn default() -> PackerConfig {
        PackerConfig {
            max_width: 1024,
            max_height: 1024,
            allow_rotation: true,
            texture_padding: 2,
            texture_extrusion: 0,
        }
    }
}

/// Defines a rectangle in pixels with the origin at the top-left of the texture atlas.
#[derive(Copy, Clone, Debug)]
pub struct Rect {
    /// Horizontal position the rectangle begins at.
    pub x: u32,
    /// Vertical position the rectangle begins at.
    pub y: u32,
    /// Width of the rectangle.
    pub w: u32,
    /// Height of the rectangle.
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

/// Boundaries and properties of a packed texture.
#[derive(Clone, Debug)]
pub struct Frame<K> {
    /// Key used to uniquely identify this frame.
    pub key: K,
    /// Rectangle describing the texture coordinates and size.
    pub uv: Rect,
    /// True if the texture was rotated during packing.
    /// If it was rotated, it was rotated 90 degrees clockwise.
    pub rotated: bool,
    /// True if the texture was trimmed during packing.
    pub trimmed: bool,

    // (x, y) is the trimmed frame position at original image
    // (w, h) is original image size
    //
    //            w
    //     +--------------+
    //     | (x, y)       |
    //     |  ^           |
    //     |  |           |
    //     |  *********   |
    //     |  *       *   |  h
    //     |  *       *   |
    //     |  *********   |
    //     |              |
    //     +--------------+
    /// Source texture size before any trimming.
    pub source: Rect,
}

pub trait Packer<K> {
    fn pack(&mut self, key: K, w: u32, h: u32) -> Option<Frame<K>>;
    fn config(&self) -> &PackerConfig;
}
