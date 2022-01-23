use crate::bitmap::store::array::simd::lut::UNIQUE_SHUF;
use crate::bitmap::store::array::util::or_array_walk_mut;
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{simd_merge, store, Shr1};
use std::mem;
use std::simd::{u16x8, u8x16, Simd, Swizzle2};

#[inline]
fn store_unique(old: u16x8, newval: u16x8, output: &mut [u16]) -> usize {
    let tmp: u16x8 = Shr1::swizzle2(newval, old);
    let mask: usize = to_bitmask(tmp.lanes_eq(newval));
    let count: usize = 8 - mask.count_ones() as usize;
    let key: u8x16 = Simd::from_slice(&UNIQUE_SHUF[mask * 16..]);
    let key: u16x8 = unsafe { mem::transmute(key) };
    let val: u16x8 = swizzle_u16x8(newval, key);
    store(val, output);
    count
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

    let v_a: u16x8 = Simd::from_slice(lhs);
    let v_b: u16x8 = Simd::from_slice(rhs);
    let (mut v_min, mut v_max) = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    let mut k = 0;
    k += store_unique(Simd::splat(u16::MAX), v_min, &mut out[k..]);
    let mut v_prev: u16x8 = v_min;
    if (i < len1) && (j < len2) {
        let mut v: u16x8;
        let mut cur_a: u16 = lhs[8 * i];
        let mut cur_b: u16 = rhs[8 * j];
        loop {
            if cur_a <= cur_b {
                v = Simd::from_slice(&lhs[8 * i..]);
                i += 1;
                if i < len1 {
                    cur_a = lhs[8 * i];
                } else {
                    break;
                }
            } else {
                v = Simd::from_slice(&rhs[8 * j..]);
                j += 1;
                if j < len2 {
                    cur_b = rhs[8 * j];
                } else {
                    break;
                }
            }
            (v_min, v_max) = simd_merge(v, v_max);
            k += store_unique(v_prev, v_min, &mut out[k..]);
            v_prev = v_min;
        }
        (v_min, v_max) = simd_merge(v, v_max);
        k += store_unique(v_prev, v_min, &mut out[k..]);
        v_prev = v_min;
    }
    // we finish the rest off using a scalar algorithm
    // could be improved?
    //
    // copy the small end on a tmp buffer
    let mut buffer: [u16; 16] = [0; 16];
    // remaining size
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
