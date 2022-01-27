use crate::bitmap::store::array::util::or_array_walk_mut;

use crate::bitmap::store::array::simd::{load, simd_merge, store, unique_swizzle, Shr1};
use core_simd::{u16x8, Simd, Swizzle2};

#[inline]
fn store_unique(old: u16x8, newval: u16x8, output: &mut [u16]) -> usize {
    let tmp: u16x8 = Shr1::swizzle2(newval, old);
    let mask = tmp.lanes_eq(newval).to_bitmask()[0];
    let count: usize = 8 - mask.count_ones() as usize;
    let val = unique_swizzle(newval, 255 - mask);
    store(val, output);
    count
}

#[inline]
#[allow(dead_code)]
fn get_debug_asserted<T>(slice: &[T], index: usize) -> T
where
    T: Copy,
{
    debug_assert!(index < slice.len());
    unsafe { *slice.get_unchecked(index) }
}

#[inline]
#[allow(dead_code)]
fn get_checked<T>(slice: &[T], index: usize) -> T
where
    T: Copy,
{
    slice[index]
}

#[inline]
fn get_idx<T>(slice: &[T], index: usize) -> T
where
    T: Copy,
{
    get_debug_asserted(slice, index)
}

/// De-duplicates `slice` in place
/// Returns the end index of the deduplicated slice.
/// elements after the return value are not guaranteed to be unique or in order
fn dedup(slice: &mut [u16]) -> usize {
    let mut pos: usize = 1;
    for i in 1..slice.len() {
        if get_idx(slice, i) != get_idx(slice, i - 1) {
            slice[pos] = get_idx(slice, i);
            pos += 1;
        }
    }
    pos
}

// a one-pass SSE union algorithm
pub fn or(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut out = vec![0; lhs.len() + rhs.len()];

    if (lhs.len() < 8) || (rhs.len() < 8) {
        let len = or_array_walk_mut(lhs, rhs, out.as_mut_slice());
        out.truncate(len);
        return out;
    }

    let len1: usize = lhs.len() / 8;
    let len2: usize = rhs.len() / 8;

    let v_a: u16x8 = load(lhs);
    let v_b: u16x8 = load(rhs);
    let [mut v_min, mut v_max] = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    let mut k = 0;
    k += store_unique(Simd::splat(u16::MAX), v_min, &mut out[k..]);
    let mut v_prev: u16x8 = v_min;
    if (i < len1) && (j < len2) {
        let mut v: u16x8;
        let mut cur_a: u16 = get_idx(lhs, 8 * i);
        let mut cur_b: u16 = get_idx(rhs, 8 * j);
        loop {
            if cur_a <= cur_b {
                v = load(&lhs[8 * i..]);
                i += 1;
                if i < len1 {
                    cur_a = get_idx(lhs, 8 * i);
                } else {
                    break;
                }
            } else {
                v = load(&rhs[8 * j..]);
                j += 1;
                if j < len2 {
                    cur_b = get_idx(rhs, 8 * j);
                } else {
                    break;
                }
            }
            [v_min, v_max] = simd_merge(v, v_max);
            k += store_unique(v_prev, v_min, &mut out[k..]);
            v_prev = v_min;
        }
        [v_min, v_max] = simd_merge(v, v_max);
        k += store_unique(v_prev, v_min, &mut out[k..]);
        v_prev = v_min;
    }

    debug_assert!(i == len1 || j == len2);

    // we finish the rest off using a scalar algorithm
    // could be improved?
    //
    // copy the small end on a tmp buffer
    let mut buffer: [u16; 16] = [0; 16];
    // remaining size
    let mut rem = store_unique(v_prev, v_max, &mut buffer);
    let (tail_a, tail_b, tail_len) = {
        if i == len1 {
            (&lhs[8 * i..], &rhs[8 * j..], lhs.len() - 8 * len1)
        } else {
            (&rhs[8 * j..], &lhs[8 * i..], rhs.len() - 8 * len2)
        }
    };
    buffer[rem..rem + tail_len].copy_from_slice(tail_a);
    rem += tail_len;
    buffer[..rem as usize].sort_unstable();
    rem = dedup(&mut buffer[..rem]);
    k += or_array_walk_mut(&buffer[..rem], tail_b, &mut out[k..]);
    out.truncate(k);
    out
}
