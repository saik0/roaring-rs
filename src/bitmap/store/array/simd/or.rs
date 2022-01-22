use crate::bitmap::store::array::simd::lut::uniqshuf;
use crate::bitmap::store::array::util::or_array_walk_mut;
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{lanes_max, lanes_min, load_unchecked, store_unchecked};
use std::mem;
use std::simd::{u16x8, Simd};

#[inline]
fn store_unique(old: u16x8, newval: u16x8, output: &mut [u16]) -> usize {
    use std::simd::{Swizzle2, Which, Which::First as A, Which::Second as B};

    /// A static swizzle
    struct UniqueSwizzle;
    impl Swizzle2<8, 8> for UniqueSwizzle {
        const INDEX: [Which; 8] = [B(7), A(0), A(1), A(2), A(3), A(4), A(5), A(6)];
    }

    let tmp: u16x8 = UniqueSwizzle::swizzle2(newval, old);
    let mask: usize = to_bitmask(tmp.lanes_eq(newval));
    let count: usize = 8 - mask.count_ones() as usize;
    let key: u16x8 =
        Simd::from_slice(&unsafe { mem::transmute::<&[u8], &[u16]>(&uniqshuf) }[mask * 8..]);
    let val: u16x8 = swizzle_u16x8(newval, key);
    unsafe {
        store_unchecked(val, output);
    }
    count
}

/// Assuming that a and b are sorted, returns a tuple of sorted output.
/// Developed originally for merge sort using SIMD instructions.
/// Standard merge. See, e.g., Inoue and Taura, SIMD- and Cache-Friendly
/// Algorithm for Sorting an Array of Structures
pub fn simd_merge(a: u16x8, b: u16x8) -> (u16x8, u16x8) {
    let mut tmp: u16x8 = lanes_min(a, b);
    let mut max: u16x8 = lanes_max(a, b);
    tmp = tmp.rotate_lanes_left::<1>();
    let mut min: u16x8 = lanes_min(tmp, max);
    for _ in 0..6 {
        max = lanes_max(tmp, max);
        tmp = min.rotate_lanes_left::<1>();
        min = lanes_min(tmp, max);
    }
    max = lanes_max(tmp, max);
    min = min.rotate_lanes_left::<1>();
    (min, max)
}

/// De-duplicates `slice` in place
/// Returns the end index of the deduplicated slice.
/// elements after the return value are not guaranteed to be unique or in order
fn dedup(slice: &mut [u16]) -> usize {
    let mut pos: usize = 1;
    for i in 1..slice.len() {
        if slice[i] != slice[i - 1] {
            slice[pos] = slice[i];
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

    let v_a: u16x8 = unsafe { load_unchecked(lhs) };
    let v_b: u16x8 = unsafe { load_unchecked(rhs) };
    let (mut v_min, mut v_max) = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    let mut k = 0;
    k += store_unique(Simd::splat(u16::MAX), v_min, &mut out[k..]);
    let mut v_prev: u16x8 = v_min;
    if (i < len1) && (j < len2) {
        let mut V: u16x8;
        let mut curA: u16 = lhs[8 * i];
        let mut curB: u16 = rhs[8 * j];
        loop {
            if curA <= curB {
                V = unsafe { load_unchecked(&lhs[8 * i..]) };
                i += 1;
                if i < len1 {
                    curA = lhs[8 * i];
                } else {
                    break;
                }
            } else {
                V = unsafe { load_unchecked(&rhs[8 * j..]) };
                j += 1;
                if j < len2 {
                    curB = rhs[8 * j];
                } else {
                    break;
                }
            }
            (v_min, v_max) = simd_merge(V, v_max);
            k += store_unique(v_prev, v_min, &mut out[k..]);
            v_prev = v_min;
        }
        (v_min, v_max) = simd_merge(V, v_max);
        k += store_unique(v_prev, v_min, &mut out[k..]);
        v_prev = v_min;
    }
    // we finish the rest off using a scalar algorithm
    // could be improved?
    //
    // copy the small end on a tmp buffer
    let mut buffer: [u16; 16] = [0; 16];
    /// remaining size
    let mut rem = store_unique(v_prev, v_max, &mut buffer);
    if i == len1 {
        let n = lhs.len() - 8 * len1;
        buffer[rem..rem + n].copy_from_slice(&lhs[8 * i..]);
        rem += n;
        buffer[..rem as usize].sort_unstable();
        rem = dedup(&mut buffer[..rem]);
        k += or_array_walk_mut(&buffer[..rem], &rhs[8 * j..], &mut out[k..]);
    } else {
        let n = rhs.len() - 8 * len2;
        buffer[rem..rem + n].copy_from_slice(&rhs[8 * j..]);
        rem += n;
        buffer[..rem as usize].sort_unstable();
        rem = dedup(&mut buffer[..rem]);
        k += or_array_walk_mut(&buffer[..rem], &lhs[8 * i..], &mut out[k..]);
    }
    out.truncate(k);
    out
}
