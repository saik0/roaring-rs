use crate::bitmap::store::array::simd::{load, simd_merge, store, unique_swizzle, Shr1, Shr2};
use crate::bitmap::store::array::xor_array_walk_mut;
use crate::bitmap::store::array_store::visitor::ArrayBinaryOperationVisitor;
use core_simd::{mask16x8, u16x8, Simd, Swizzle2};

// write vector new, while omitting repeated values assuming that previously
// written vector was "old"
#[inline]
fn store_unique_xor(old: u16x8, new: u16x8, f: impl FnOnce(u16x8, u8)) {
    let tmp1: u16x8 = Shr2::swizzle2(new, old);
    let tmp2: u16x8 = Shr1::swizzle2(new, old);
    let eq_l: mask16x8 = tmp2.lanes_eq(tmp1);
    let eq_r: mask16x8 = tmp2.lanes_eq(new);
    let eq_l_or_r: mask16x8 = eq_l | eq_r;
    let mask: u8 = eq_l_or_r.to_bitmask()[0];
    f(tmp2, 255 - mask);
}

/// De-duplicates `slice` in place, removing _both_ duplicates
/// Returns the end index of the xor-ed slice.
/// elements after the return value are not guaranteed to be unique or in order
/// #[inline]
fn xor_slice(slice: &mut [u16]) -> usize {
    let mut pos: usize = 1;
    for i in 1..slice.len() {
        if slice[i] != slice[i - 1] {
            slice[pos] = slice[i];
            pos += 1;
        } else {
            pos -= 1; // it is identical to previous, delete it
        }
    }
    pos
}

// a one-pass SSE xor algorithm
pub fn xor(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    if (lhs.len() < 8) || (rhs.len() < 8) {
        xor_array_walk(lhs, rhs, visitor);
        return;
    }

    let len1: usize = lhs.len() / 8;
    let len2: usize = rhs.len() / 8;

    let v_a: u16x8 = load(lhs);
    let v_b: u16x8 = load(rhs);
    let [mut v_min, mut v_max] = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    store_unique_xor(Simd::splat(u16::MAX), v_min, |v, m| visitor.visit_vector(v, m));
    let mut v_prev: u16x8 = v_min;
    if (i < len1) && (j < len2) {
        let mut v: u16x8;
        let mut cur_a: u16 = lhs[8 * i];
        let mut cur_b: u16 = rhs[8 * j];
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
            store_unique_xor(v_prev, v_min, |v, m| visitor.visit_vector(v, m));
            v_prev = v_min;
        }
        [v_min, v_max] = simd_merge(v, v_max);
        store_unique_xor(v_prev, v_min, |v, m| visitor.visit_vector(v, m));
        v_prev = v_min;
    }

    debug_assert!(i == len1 || j == len2);

    // we finish the rest off using a scalar algorithm
    // could be improved?
    // conditionally stores the last value of laststore as well as all but the
    // last value of vecMax,
    let mut buffer: [u16; 17] = [0; 17];
    // remaining size
    let mut rem = 0;
    store_unique_xor(v_prev, v_max, |v, m| {
        store(unique_swizzle(v, m), buffer.as_mut_slice());
        rem = m.count_ones() as usize;
    });

    let arr_max = v_max.as_array();
    let vec7 = arr_max[7];
    let vec6 = arr_max[6];
    if vec6 != vec7 {
        buffer[rem] = vec7;
        rem += 1;
    }

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
        rem = xor_slice(&mut buffer[..rem]);
        xor_array_walk(&buffer[..rem], tail_b, visitor);
    }
}

pub fn xor_array_walk(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    use std::cmp::Ordering;
    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Ordering::Less => {
                visitor.visit_scalar(*a);
                i += 1;
            }
            Ordering::Greater => {
                visitor.visit_scalar(*b);
                j += 1;
            }
            Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }

    visitor.visit_slice(&lhs[i..]);
    visitor.visit_slice(&rhs[j..]);
}
