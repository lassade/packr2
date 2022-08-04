use super::{Frame, Packer, PackerConfig, Rect};

struct created_splits {
    count: usize,
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
            count: core::usize::MAX,
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

    const fn is_failed(&self) -> bool {
        self.count != core::usize::MAX
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
}
