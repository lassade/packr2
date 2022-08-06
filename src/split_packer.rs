use crate::{Packer, PackerConfig, Rect, Rectf, Size};

struct Splits {
    count: u32,
    spaces: [Rect; 2],
}

impl From<Rect> for Splits {
    fn from(space: Rect) -> Self {
        Self {
            count: 1,
            spaces: [
                space,
                // safety: `Rect` is just made out of unsigned integers and the second one doesnt need to be initialized
                unsafe { core::mem::MaybeUninit::uninit().assume_init() },
            ],
        }
    }
}

impl From<[Rect; 2]> for Splits {
    fn from(spaces: [Rect; 2]) -> Self {
        Self { count: 2, spaces }
    }
}

impl Splits {
    const fn failed() -> Self {
        Self {
            count: core::u32::MAX,
            // safety: spaces are invalid
            spaces: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
        }
    }

    const fn none() -> Self {
        Self {
            count: 0,
            // safety: there is no spaces in this split
            spaces: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
        }
    }

    const fn better_than(&self, b: &Splits) -> bool {
        self.count < b.count
    }

    const fn is_valid(&self) -> bool {
        self.count <= 2
    }
}

#[inline]
fn insert_and_split(w: u32, h: u32, space_available: &Rect /* Space rectangle */) -> Splits {
    if space_available.w < w || space_available.h < h {
        // Image is bigger than the candidate empty space.
        // We'll need to look further.
        return Splits::failed();
    }

    // sp is always greater than [w, h]
    let free_w = space_available.w.wrapping_sub(w);
    let free_h = space_available.h.wrapping_sub(h);

    if free_w == 0 && free_h == 0 {
        // If the image dimensions equal the dimensions of the candidate empty space (image fits exactly),
        // we will just delete the space and create no splits.
        return Splits::none();
    }

    // If the image fits into the candidate empty space,
    // but exactly one of the image dimensions equals the respective dimension of the candidate empty space
    // (e.g. image = 20x40, candidate space = 30x40)
    // we delete the space and create a single split. In this case a 10x40 space.
    if free_w > 0 && free_h == 0 {
        let mut r = space_available.clone();
        r.x += w;
        r.w -= w;
        return r.into();
    }

    if free_w == 0 && free_h > 0 {
        let mut r = space_available.clone();
        r.y += h;
        r.h -= h;
        return r.into();
    }

    // Every other option has been exhausted,
    // so at this point the image must be *strictly* smaller than the empty space,
    // that is, it is smaller in both width and height.
    //
    // Thus, free_w and free_h must be positive.

    // Decide which way to split.
    //
    // Instead of having two normally-sized spaces,
    // it is better - though I have no proof of that - to have a one tiny space and a one huge space.
    // This creates better opportunity for insertion of future rectangles.
    //
    // This is why, if we had more of width remaining than we had of height,
    // we split along the vertical axis,
    // and if we had more of height remaining than we had of width,
    // we split along the horizontal axis.
    if free_w > free_h {
        let bigger_split = Rect {
            x: space_available.x + w,
            y: space_available.y,
            w: free_w,
            h: space_available.h,
        };

        let lesser_split = Rect {
            x: space_available.x,
            y: space_available.y + h,
            w: w,
            h: free_h,
        };

        return [bigger_split, lesser_split].into();
    }

    let bigger_split = Rect {
        x: space_available.x,
        y: space_available.y + h,
        w: space_available.w,
        h: free_h,
    };

    let lesser_split = Rect {
        x: space_available.x + w,
        y: space_available.y,
        w: free_w,
        h: h,
    };

    return [bigger_split, lesser_split].into();
}

/// Derived from the [lightmap packer](https://blackpawn.com/texts/lightmaps/default.html)
/// but uses a vector instead a tree, sourced from [`rectpack2D`](https://github.com/TeamHypersomnia/rectpack2D)
pub struct SplitPacker {
    current_aabb: Size,
    spaces: Vec<Rect>,
    config: PackerConfig,
}

impl SplitPacker {
    pub fn new(config: PackerConfig) -> Self {
        let mut tmp = Self {
            current_aabb: Size { w: 0, h: 0 },
            spaces: vec![],
            config,
        };
        tmp.spaces.push(Rect {
            x: 0,
            y: 0,
            w: config.max_width,
            h: config.max_width,
        });
        tmp
    }

    pub const fn get_rects_aabb(&self) -> Size {
        self.current_aabb
    }

    // #[inline]
    // fn get_spaces(&self) -> &[Rect] {
    //     &self.spaces[..]
    // }
}

impl Packer for SplitPacker {
    fn insert(&mut self, w: u32, h: u32) -> Option<Rectf> {
        for i in (0..self.spaces.len()).rev() {
            let candidate_space = self.spaces[i];

            let normal = insert_and_split(w, h, &candidate_space);

            let mut accept_insert = |splits: &Splits, flipped| -> Option<Rectf> {
                self.spaces.remove(i);

                for s in 0..splits.count as usize {
                    // note: it can never fail to insert more spaces, but if it does you must return `None` here!
                    self.spaces.push(splits.spaces[s]);
                }

                let r = if flipped {
                    Rectf {
                        x: candidate_space.x,
                        y: candidate_space.y,
                        w: h,
                        h: w,
                        flipped,
                    }
                } else {
                    Rectf {
                        x: candidate_space.x,
                        y: candidate_space.y,
                        w,
                        h,
                        flipped,
                    }
                };

                self.current_aabb.expand_with(&r);

                Some(r)
            };

            if self.config.allow_flipping {
                let flipped = insert_and_split(h, w, &candidate_space);

                match (normal.is_valid(), flipped.is_valid()) {
                    (true, true) => {
                        // if both were successful, prefer the one that generated less remainder spaces.
                        if flipped.better_than(&normal) {
                            // Accept the flipped result if it producues less or "better" spaces.
                            return (accept_insert)(&flipped, true);
                        }

                        return (accept_insert)(&normal, false);
                    }
                    (true, _) => {
                        return (accept_insert)(&normal, false);
                    }
                    (_, true) => {
                        return (accept_insert)(&flipped, true);
                    }
                    _ => {}
                }
            } else {
                if normal.is_valid() {
                    return (accept_insert)(&normal, false);
                }
            }
        }

        None
    }

    fn reset(&mut self) {
        self.current_aabb = Size { w: 0, h: 0 };
        self.spaces.clear();
        self.spaces.push(Rect {
            x: 0,
            y: 0,
            w: self.config.max_width,
            h: self.config.max_height,
        });
    }
}
