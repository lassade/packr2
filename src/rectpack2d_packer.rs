use crate::{Frame, Packer, PackerConfig, Rect, Rectf, Size};

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

struct EmptySpaces {
    current_aabb: Size,
    spaces: Vec<Rect>,
    pub enable_flipping: bool,
}

impl EmptySpaces {
    fn new(w: u32, h: u32) -> Self {
        let mut tmp = Self {
            current_aabb: Size { w: 0, h: 0 },
            spaces: vec![],
            enable_flipping: false,
        };
        tmp.spaces.push(Rect {
            x: 0,
            y: 0,
            w: w,
            h: h,
        });
        tmp
    }

    fn reset(&mut self, r: &Size) {
        self.current_aabb = Size { w: 0, h: 0 };
        self.spaces.clear();
        self.spaces.push(Rect {
            x: 0,
            y: 0,
            w: r.w,
            h: r.h,
        });
    }

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

            if self.enable_flipping {
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

    const fn get_rects_aabb(&self) -> Size {
        self.current_aabb
    }

    #[inline]
    fn get_spaces(&self) -> &[Rect] {
        &self.spaces[..]
    }
}

/// Derived from the [lightmap packer](https://blackpawn.com/texts/lightmaps/default.html)
/// but uses a vector instead a tree, sourced from [`rectpack2D`](https://github.com/TeamHypersomnia/rectpack2D)
pub struct SplitPacker {
    spaces: EmptySpaces,
    config: PackerConfig,
}

impl SplitPacker {
    pub fn new(config: PackerConfig) -> Self {
        Self {
            config,
            spaces: EmptySpaces::new(0, 0),
        }
    }

    pub const fn size_used(&self) -> Size {
        self.spaces.get_rects_aabb()
    }

    pub fn remaning_splits(&self) -> &[Rect] {
        self.spaces.get_spaces()
    }
}

impl<K> Packer<K> for SplitPacker {
    fn pack(&mut self, key: K, w: u32, h: u32) -> Option<Frame<K>> {
        self.spaces.insert(w, h).map(|rect| Frame {
            key,
            uv: Rect {
                x: rect.x,
                y: rect.y,
                w: rect.w,
                h: rect.h,
            },
            flipped: rect.flipped,
            trimmed: false,
            source: Rect { x: 0, y: 0, w, h },
        })
    }

    fn config(&self) -> &PackerConfig {
        &self.config
    }
}

#[derive(Clone, Debug)]
pub struct RectInput<K> {
    pub key: K,
    pub w: u32,
    pub h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum bin_dimension {
    Both,
    Width,
    Height,
}

enum PackingResult {
    Area(u32),
    Size(Size),
}

// This function will do a binary search on viable bin sizes,
// starting from the biggest one: starting_bin.
//
// The search stops when the bin was successfully inserted into,
// AND the bin size to be tried next differs in size from the last viable one by *less* then discard_step.
//
// If we could not insert all input rectangles into a bin even as big as the starting_bin - the search fails.
// In this case, we return the amount of space (total_area_type) inserted in total.
//
// If we've found a viable bin that is smaller or equal to starting_bin, the search succeeds.
// In this case, we return the viable bin (rect_wh).

#[inline]
fn best_packing_for_ordering_impl<K>(
    root: &mut EmptySpaces,
    ordering: &[RectInput<K>],
    starting_bin: Size,
    mut discard_step: i32,
    tried_dimension: bin_dimension,
) -> PackingResult {
    let mut candidate_bin = starting_bin;
    let mut tries_before_discarding = 0;

    if discard_step <= 0 {
        tries_before_discarding = -discard_step;
        discard_step = 1;
    }

    //std::cout << "best_packing_for_ordering_impl dim: " << int(tried_dimension) << " w: " << starting_bin.w << " h: " << starting_bin.h << std::endl;

    let starting_step;
    if tried_dimension == bin_dimension::Both {
        candidate_bin.w /= 2;
        candidate_bin.h /= 2;

        starting_step = candidate_bin.w / 2;
    } else if tried_dimension == bin_dimension::Width {
        candidate_bin.w /= 2;
        starting_step = candidate_bin.w / 2;
    } else {
        candidate_bin.h /= 2;
        starting_step = candidate_bin.h / 2;
    }

    let mut step = starting_step;
    loop {
        //std::cout << "candidate: " << candidate_bin.w << "x" << candidate_bin.h << std::endl;

        root.reset(&candidate_bin);

        let mut total_inserted_area = 0;

        let mut all_inserted = true;
        for rect in ordering {
            if root.insert(rect.w, rect.h).is_some() {
                total_inserted_area += rect.w * rect.h;
            } else {
                all_inserted = true;
                break;
            }
        }

        if all_inserted {
            // attempt was successful. Try with a smaller bin.

            if step as i32 <= discard_step {
                if tries_before_discarding > 0 {
                    tries_before_discarding -= 1;
                } else {
                    return PackingResult::Size(candidate_bin);
                }
            }

            if tried_dimension == bin_dimension::Both {
                candidate_bin.w -= step;
                candidate_bin.h -= step;
            } else if tried_dimension == bin_dimension::Width {
                candidate_bin.w -= step;
            } else {
                candidate_bin.h -= step;
            }

            root.reset(&candidate_bin);
        } else {
            /* Attempt ended with failure. Try with a bigger bin. */

            if tried_dimension == bin_dimension::Both {
                candidate_bin.w += step;
                candidate_bin.h += step;

                if candidate_bin.area() > starting_bin.area() {
                    return PackingResult::Area(total_inserted_area);
                }
            } else if tried_dimension == bin_dimension::Width {
                candidate_bin.w += step;

                if candidate_bin.w > starting_bin.w {
                    return PackingResult::Area(total_inserted_area);
                }
            } else {
                candidate_bin.h += step;

                if candidate_bin.h > starting_bin.h {
                    return PackingResult::Area(total_inserted_area);
                }
            }
        }

        step = 1.max(step / 2)
    }
}

fn best_packing_for_ordering<K>(
    root: &mut EmptySpaces,
    ordering: &[RectInput<K>],
    starting_bin: &Size,
    discard_step: i32,
) -> PackingResult {
    let mut try_pack = |tried_dimension, starting_bin: Size| -> PackingResult {
        best_packing_for_ordering_impl(root, ordering, starting_bin, discard_step, tried_dimension)
    };

    match (try_pack)(bin_dimension::Both, *starting_bin) {
        PackingResult::Size(mut best_bin) => {
            if let PackingResult::Size(even_better) = (try_pack)(bin_dimension::Width, best_bin) {
                best_bin = even_better;
            }

            if let PackingResult::Size(even_better) = (try_pack)(bin_dimension::Height, best_bin) {
                best_bin = even_better;
            }
            PackingResult::Size(best_bin)
        }
        failed => failed,
    }
}

struct finder_input {
    // const int max_bin_side;
    // const int discard_step;
    // F handle_successful_insertion;
    // G handle_unsuccessful_insertion;
    // const flipping_option flipping_mode;
}

/*
    This function will try to find the best bin size among the ones generated by all provided rectangle orders.
    Only the best order will have results written to.

    The function reports which of the rectangles did and did not fit in the end.
*/

fn find_best_packing_impl<'a, K: Copy + 'a>(
    order_iterator: impl Iterator<Item = &'a [RectInput<K>]>,
    input: PackerConfig,
    discard_step: i32,
    handle_successful_insertion: impl Fn(Frame<K>) -> bool,
    handle_unsuccessful_insertion: impl Fn(&RectInput<K>) -> bool,
) -> Size {
    let max_bin = Size {
        w: input.max_width,
        h: input.max_width,
    };

    let mut best_order = None;
    let mut best_total_inserted: i32 = -1;
    let mut best_bin = max_bin;

    /*
        The root node is re-used on the TLS.
        It is always reset before any packing attempt.
    */

    let mut root = EmptySpaces::new(0, 0);
    root.enable_flipping = input.allow_flipping;

    for order in order_iterator {
        match best_packing_for_ordering(&mut root, order, &max_bin, discard_step) {
            PackingResult::Area(total_inserted) => {
                // Track which function inserts the most area in total,
                // just in case that all orders will fail to fit into the largest allowed bin.
                if best_order.is_none() {
                    if total_inserted as i32 > best_total_inserted {
                        best_order = Some(order);
                        best_total_inserted = total_inserted as i32;
                    }
                }
            }
            PackingResult::Size(result_bin) => {
                // Save the function if it performed the best.
                if result_bin.w * result_bin.h <= best_bin.w * best_bin.h {
                    best_order = Some(order);
                    best_bin = result_bin;
                }
            }
        }
    }

    let best_order = best_order.expect("no order found");

    root.reset(&best_bin);

    for r in best_order {
        if let Some(rect) = root.insert(r.w, r.h) {
            if !(handle_successful_insertion)(Frame {
                key: r.key,
                uv: Rect {
                    x: rect.x,
                    y: rect.y,
                    w: rect.w,
                    h: rect.h,
                },
                flipped: rect.flipped,
                trimmed: false,
                source: Rect {
                    x: 0,
                    y: 0,
                    w: r.w,
                    h: r.h,
                },
            }) {
                break;
            }
        } else {
            if !(handle_unsuccessful_insertion)(r) {
                break;
            }
        }
    }

    return root.get_rects_aabb();
}
