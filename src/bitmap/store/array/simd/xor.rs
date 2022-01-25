use crate::bitmap::store::array::simd::lut::UNIQUE_SHUF;
use crate::bitmap::store::array::xor_array_walk_mut;
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{simd_merge, store, Shr1, Shr2};
use core_simd::{mask16x8, u16x8, u8x16, Simd, Swizzle2};
use std::mem;

// write vector new, while omitting repeated values assuming that previously
// written vector was "old"
#[inline]
fn store_unique_xor(old: u16x8, new: u16x8, output: &mut [u16]) -> usize {
    let tmp1: u16x8 = Shr2::swizzle2(new, old);
    let tmp2: u16x8 = Shr1::swizzle2(new, old);
    let eq_l: mask16x8 = tmp2.lanes_eq(tmp1);
    let eq_r: mask16x8 = tmp2.lanes_eq(new);
    let eq_l_or_r: mask16x8 = eq_l | eq_r;
    let mask: usize = to_bitmask(eq_l_or_r);
    let count: usize = 8 - mask.count_ones() as usize;
    let key: u8x16 = Simd::from_slice(&UNIQUE_SHUF[mask * 16..]);
    let key: u16x8 = unsafe { mem::transmute(key) };
    let val: u16x8 = swizzle_u16x8(tmp2, key);
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

    if (lhs.len() < 8) || (rhs.len() < 8) {
        let len = xor_array_walk_mut(lhs, rhs, out.as_mut_slice());
        out.truncate(len);
        return out;
    }

    let len1: usize = lhs.len() / 8;
    let len2: usize = rhs.len() / 8;

    let v_a: u16x8 = Simd::from_slice(lhs);
    let v_b: u16x8 = Simd::from_slice(rhs);
    let [mut v_min, mut v_max] = simd_merge(v_a, v_b);

    let mut i = 1;
    let mut j = 1;
    let mut k = 0;
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
    // remaining size
    let mut rem = store_unique_xor(v_prev, v_max, &mut buffer);
    let arr_max = v_max.as_array();
    let vec7 = arr_max[7];
    let vec6 = arr_max[6];
    if vec6 != vec7 {
        buffer[rem] = vec7;
        rem += 1;
    }
    if i == len1 {
        let n = lhs.len() - 8 * len1;
        buffer[rem..rem + n].copy_from_slice(&lhs[8 * i..]);
        rem += n;
        if rem == 0 {
            // trivial case
            out[k..k + len2].copy_from_slice(&rhs[j..j + len2]);
            k += len2 - j;
            // memcpy(output, array2 + 8 * pos2,
            //        (length2 - 8 * pos2) * sizeof(uint16_t));
            // len += (length2 - 8 * pos2);
        } else {
            buffer[..rem as usize].sort_unstable();
            rem = xor_slice(&mut buffer[..rem]);
            k += xor_array_walk_mut(&buffer[..rem], &rhs[8 * j..], &mut out[k..]);
        }
    } else {
        let n = rhs.len() - 8 * len2;
        buffer[rem..rem + n].copy_from_slice(&rhs[8 * j..]);
        rem += n;
        if rem == 0 {
            // trivial case
            out[k..k + len1].copy_from_slice(&lhs[i..i + len1]);
            k += len1 - i;
            // memcpy(output, array1 + 8 * pos1,
            //        (length1 - 8 * pos1) * sizeof(uint16_t));
            // len += (length1 - 8 * pos1);
        } else {
            buffer[..rem as usize].sort_unstable();
            rem = xor_slice(&mut buffer[..rem]);
            k += xor_array_walk_mut(&buffer[..rem], &lhs[8 * i..], &mut out[k..]);
        }
    }
    out.truncate(k);
    out
}
