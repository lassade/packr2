use crate::{Packer, Size};

// find best packing implementation

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

#[inline(always)]
fn best_packing_for_ordering_impl<P: Packer, K>(
    root: &mut P,
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

        root.reset(Some(candidate_bin));

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

            root.reset(Some(candidate_bin));
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

fn best_packing_for_ordering<P: Packer, K>(
    root: &mut P,
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
        h: input.max_height,
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
            if !(handle_successful_insertion)(rect) {
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
