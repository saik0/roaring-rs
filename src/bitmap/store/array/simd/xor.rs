use crate::bitmap::store::array::simd::lut::unique_swizzle;
use crate::bitmap::store::array::xor_array_walk_mut;
use crate::simd::util::{simd_merge, store, Shr1, Shr2};
use core_simd::{mask16x8, u16x8, Simd, Swizzle2};

// write vector new, while omitting repeated values assuming that previously
// written vector was "old"
#[inline]
fn store_unique_xor(old: u16x8, new: u16x8, output: &mut [u16]) -> usize {
    let tmp1: u16x8 = Shr2::swizzle2(new, old);
    let tmp2: u16x8 = Shr1::swizzle2(new, old);
    let eq_l: mask16x8 = tmp2.lanes_eq(tmp1);
    let eq_r: mask16x8 = tmp2.lanes_eq(new);
    let eq_l_or_r: mask16x8 = eq_l | eq_r;
    let mask: u8 = eq_l_or_r.to_bitmask()[0];
    let count: usize = 8 - mask.count_ones() as usize;
    let val: u16x8 = unique_swizzle(tmp2, 255 - mask);
    store(val, output);
    count
}

/// De-duplicates `slice` in place, removing _both_ duplicates
/// Returns the end index of the xor-ed slice.
/// elements after the return value are not guaranteed to be unique or in order
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
pub fn xor(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut out = vec![0; lhs.len() + rhs.len()];
    let mut k = 0;

    if (lhs.len() < 8) || (rhs.len() < 8) {
        k += xor_array_walk_mut(lhs, rhs, &mut out[k..]);
        out.truncate(k);
        return out;
    }

    let len1: usize = lhs.len() / 8;
    let len2: usize = rhs.len() / 8;

    let v_a: u16x8 = Simd::from_slice(lhs);
    let v_b: u16x8 = Simd::from_slice(rhs);
    let [mut v_min, mut v_max] = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    let mut v_prev: u16x8 = v_min;
    k += store_unique_xor(Simd::splat(u16::MAX), v_min, &mut out[k..]);

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
            [v_min, v_max] = simd_merge(v, v_max);
            k += store_unique_xor(v_prev, v_min, &mut out[k..]);
            v_prev = v_min;
        }
        [v_min, v_max] = simd_merge(v, v_max);
        k += store_unique_xor(v_prev, v_min, &mut out[k..]);
        v_prev = v_min;
    }

    // we finish the rest off using a scalar algorithm
    // could be improved?
    // conditionally stores the last value of laststore as well as all but the
    // last value of vecMax,
    //
    // TODO: 17? WHY?!?
    let mut buffer: [u16; 17] = [0; 17];
    let mut rem = store_unique_xor(v_prev, v_max, &mut buffer);
    let arr_max = v_max.as_array();
    let vec7 = arr_max[7];
    let vec6 = arr_max[6];
    if vec6 != vec7 {
        buffer[rem] = vec7;
        rem += 1;
    }
    if i == len1 {
        buffer[rem..lhs.len() - 8 * len1 + rem]
            .copy_from_slice(&lhs[8 * i..8 * i + (lhs.len() - 8 * len1)]);
        rem += lhs.len() - 8 * len1;
        if rem == 0 {
            out[k..k + rhs.len() - 8 * j].copy_from_slice(&rhs[8 * j..(8 * j) + rhs.len() - 8 * j]);
            k += rhs.len() - 8 * j;
        } else {
            buffer[..rem as usize].sort_unstable();
            rem = xor_slice(&mut buffer[..rem]);
            k += xor_array_walk_mut(
                &buffer[..rem],
                &rhs[8 * j..(8 * j) + rhs.len() - 8 * j],
                &mut out[k..],
            );
        }
    } else {
        buffer[rem..rhs.len() - 8 * len2 + rem]
            .copy_from_slice(&rhs[8 * j..8 * j + (rhs.len() - 8 * len2)]);
        rem += rhs.len() - 8 * len2;
        if rem == 0 {
            out[k..k + (lhs.len() - 8 * i)]
                .copy_from_slice(&lhs[8 * i..(8 * i) + lhs.len() - 8 * i]);
            k += lhs.len() - 8 * i;
        } else {
            buffer[..rem as usize].sort_unstable();
            rem = xor_slice(&mut buffer[..rem]);
            k += xor_array_walk_mut(
                &buffer[..rem],
                &lhs[8 * i..(8 * i) + lhs.len() - 8 * i],
                &mut out[k..],
            );
        }
    }

    out.truncate(k);
    out
}
