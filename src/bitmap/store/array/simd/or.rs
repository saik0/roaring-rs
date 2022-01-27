use std::cmp::Ordering;

use crate::bitmap::store::array::simd::{load, simd_merge, store, unique_swizzle, Shr1};
use crate::bitmap::store::array_store::visitor::ArrayBinaryOperationVisitor;
use core_simd::{u16x8, Simd, Swizzle2};

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

#[inline]
fn store_unique(old: u16x8, new: u16x8, f: impl FnOnce(u16x8, u8)) {
    let tmp: u16x8 = Shr1::swizzle2(new, old);
    let mask = 255 - tmp.lanes_eq(new).to_bitmask()[0];
    f(new, mask);
}

// a one-pass SSE union algorithm
pub fn or(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    if (lhs.len() < 8) || (rhs.len() < 8) {
        or_array_walk(lhs, rhs, visitor);
        return;
    }

    let len1: usize = lhs.len() / 8;
    let len2: usize = rhs.len() / 8;

    let v_a: u16x8 = load(lhs);
    let v_b: u16x8 = load(rhs);
    let [mut v_min, mut v_max] = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    store_unique(Simd::splat(u16::MAX), v_min, |v, m| visitor.visit_vector(v, m));
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
                    cur_a = lhs[8 * i];
                } else {
                    break;
                }
            } else {
                v = load(&rhs[8 * j..]);
                j += 1;
                if j < len2 {
                    cur_b = rhs[8 * j];
                } else {
                    break;
                }
            }
            [v_min, v_max] = simd_merge(v, v_max);
            store_unique(v_prev, v_min, |v, m| visitor.visit_vector(v, m));
            v_prev = v_min;
        }
        [v_min, v_max] = simd_merge(v, v_max);
        store_unique(v_prev, v_min, |v, m| visitor.visit_vector(v, m));
        v_prev = v_min;
    }

    debug_assert!(i == len1 || j == len2);

    // we finish the rest off using a scalar algorithm
    // could be improved?
    //
    // copy the small end on a tmp buffer
    let mut buffer: [u16; 16] = [0; 16];
    let mut rem = 0;
    store_unique(v_prev, v_max, |v, m| {
        store(unique_swizzle(v, m), buffer.as_mut_slice());
        rem = m.count_ones() as usize;
    });

    let (tail_a, tail_b, tail_len) = if i == len1 {
        (&lhs[8 * i..], &rhs[8 * j..], lhs.len() - 8 * len1)
    } else {
        (&rhs[8 * j..], &lhs[8 * i..], rhs.len() - 8 * len2)
    };

    buffer[rem..rem + tail_len].copy_from_slice(tail_a);
    rem += tail_len;

    if rem == 0 {
        visitor.visit_slice(tail_b)
    } else {
        buffer[..rem as usize].sort_unstable();
        rem = dedup(&mut buffer[..rem]);
        or_array_walk(&buffer[..rem], tail_b, visitor);
    }
}

fn or_array_walk(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = lhs[i];
        let b = rhs[j];
        match a.cmp(&b) {
            Ordering::Less => {
                visitor.visit_scalar(a);
                i += 1;
            }
            Ordering::Greater => {
                visitor.visit_scalar(b);
                j += 1;
            }
            Ordering::Equal => {
                visitor.visit_scalar(a);
                i += 1;
                j += 1;
            }
        }
    }

    if i < lhs.len() {
        visitor.visit_slice(&lhs[i..]);
    } else if j < rhs.len() {
        visitor.visit_slice(&rhs[j..]);
    }
}
