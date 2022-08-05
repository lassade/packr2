use std::marker::PhantomData;

use super::{Frame, Packer, PackerConfig, Rect};

struct created_splits {
    count: u32,
    spaces: [Rect; 2],
}

impl From<Rect> for created_splits {
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

impl From<[Rect; 2]> for created_splits {
    fn from(spaces: [Rect; 2]) -> Self {
        Self { count: 2, spaces }
    }
}

impl created_splits {
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

    const fn better_than(&self, b: &created_splits) -> bool {
        self.count < b.count
    }

    const fn is_valid(&self) -> bool {
        self.count <= 2
    }
}

#[inline]
fn insert_and_split(
    w: u32,    /* Image rectangle */
    h: u32,    /* Image rectangle */
    sp: &Rect, /* Space rectangle */
) -> created_splits {
    let free_w = sp.w - w;
    let free_h = sp.h - h;

    if free_w < 0 || free_h < 0 {
        /*
            Image is bigger than the candidate empty space.
            We'll need to look further.
        */
        return created_splits::failed();
    }

    if free_w == 0 && free_h == 0 {
        /*
            If the image dimensions equal the dimensions of the candidate empty space (image fits exactly),
            we will just delete the space and create no splits.
        */
        return created_splits::none();
    }

    /*
        If the image fits into the candidate empty space,
        but exactly one of the image dimensions equals the respective dimension of the candidate empty space
        (e.g. image = 20x40, candidate space = 30x40)
        we delete the space and create a single split. In this case a 10x40 space.
    */
    if free_w > 0 && free_h == 0 {
        let mut r = sp.clone();
        r.x += w;
        r.w -= w;
        return r.into();
    }

    if free_w == 0 && free_h > 0 {
        let mut r = sp.clone();
        r.y += h;
        r.h -= h;
        return r.into();
    }

    /*
        Every other option has been exhausted,
        so at this point the image must be *strictly* smaller than the empty space,
        that is, it is smaller in both width and height.

        Thus, free_w and free_h must be positive.
    */
    /*
        Decide which way to split.

        Instead of having two normally-sized spaces,
        it is better - though I have no proof of that - to have a one tiny space and a one huge space.
        This creates better opportunity for insertion of future rectangles.

        This is why, if we had more of width remaining than we had of height,
        we split along the vertical axis,
        and if we had more of height remaining than we had of width,
        we split along the horizontal axis.
    */
    if free_w > free_h {
        let bigger_split = Rect {
            x: sp.x + w,
            y: sp.y,
            w: free_w,
            h: sp.h,
        };

        let lesser_split = Rect {
            x: sp.x,
            y: sp.y + h,
            w,
            h: free_h,
        };

        return [bigger_split, lesser_split].into();
    }

    let bigger_split = Rect {
        x: sp.x,
        y: sp.y + h,
        w: sp.w,
        h: free_h,
    };

    let lesser_split = Rect {
        x: sp.x + w,
        y: sp.y,
        w: free_w,
        h: h,
    };

    return [bigger_split, lesser_split].into();
}

#[derive(Default)]
pub struct empty_spaces_provider {
    empty_spaces: Vec<Rect>,
}

impl empty_spaces_provider {
    pub fn remove(&mut self, index: usize) {
        self.empty_spaces.swap_remove(index);
    }

    pub fn add(&mut self, r: Rect) -> bool {
        self.empty_spaces.push(r);
        true
    }

    pub fn get_count(&self) -> usize {
        self.empty_spaces.len()
    }

    pub fn reset(&mut self) {
        self.empty_spaces.clear();
    }

    pub fn get(&self, index: usize) -> &Rect {
        &self.empty_spaces[index]
    }
}

#[derive(Default)]
struct rect_wh {
    w: u32,
    h: u32,
}

impl rect_wh {
    const fn new(w: u32, h: u32) -> Self {
        Self { w, h }
    }

    fn flip(&mut self) -> &Self {
        core::mem::swap(&mut self.w, &mut self.h);
        self
    }

    const fn max_side(&self) -> u32 {
        if self.h > self.w {
            self.h
        } else {
            self.w
        }
    }

    const fn min_side(&self) -> u32 {
        if self.h < self.w {
            self.h
        } else {
            self.w
        }
    }

    const fn area(&self) -> u32 {
        self.w * self.h
    }

    const fn perimeter(&self) -> u32 {
        2 * self.w + 2 * self.h
    }

    fn pathological_mult(&self) -> f32 {
        self.max_side() as f32 / self.min_side() as f32 * self.area() as f32
    }

    fn expand_with(&mut self, r: &rect_xywhf) {
        self.w = self.w.max(r.x + r.w);
        self.h = self.h.max(r.y + r.h);
    }
}

pub struct rect_xywhf {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    flipped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum flipping_option {
    Disabled,
    Enabled,
}

pub struct empty_spaces {
    current_aabb: rect_wh,
    spaces: empty_spaces_provider,
    pub flipping_mode: flipping_option,
}

impl empty_spaces {
    pub fn new(w: u32, h: u32) -> Self {
        let mut tmp = Self {
            current_aabb: rect_wh { w: 0, h: 0 },
            spaces: empty_spaces_provider::default(),
            flipping_mode: flipping_option::Disabled,
        };
        tmp.spaces.add(Rect {
            x: 0,
            y: 0,
            w: w,
            h: h,
        });
        tmp
    }

    pub fn reset(&mut self, r: &rect_wh) {
        self.current_aabb = rect_wh { w: 0, h: 0 };
        self.spaces.reset();
        self.spaces.add(Rect {
            x: 0,
            y: 0,
            w: r.w,
            h: r.h,
        });
    }

    pub fn insert(&mut self, w: u32, h: u32) -> Option<rect_xywhf> {
        for i in (0..self.spaces.get_count()).rev() {
            let candidate_space = *self.spaces.get(i);

            let normal = insert_and_split(w, h, &candidate_space);

            let mut accept_insert = |splits: &created_splits, flipped| -> Option<rect_xywhf> {
                self.spaces.remove(i);

                for s in 0..splits.count as usize {
                    if !self.spaces.add(splits.spaces[s]) {
                        return None;
                    }
                }

                let r = if flipped {
                    rect_xywhf {
                        x: candidate_space.x,
                        y: candidate_space.y,
                        w: h,
                        h: w,
                        flipped,
                    }
                } else {
                    rect_xywhf {
                        x: candidate_space.x,
                        y: candidate_space.y,
                        w: h,
                        h: w,
                        flipped,
                    }
                };

                self.current_aabb.expand_with(&r);

                Some(r)
            };

            if self.flipping_mode == flipping_option::Enabled {
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

    pub const fn get_rects_aabb(&self) -> [u32; 2] {
        [self.current_aabb.w, self.current_aabb.h]
    }

    pub const fn get_spaces(&self) -> &empty_spaces_provider {
        &self.spaces
    }
}
