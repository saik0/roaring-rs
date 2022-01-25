use crate::bitmap::store::array::simd::lut::{unique_swizzle, SHUFFLE_MASK};
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{matrix_cmp, store};
use core_simd::{u16x8, u8x16, Simd};
use std::cmp::Ordering::{Greater, Less};
use std::mem;

pub fn sub(mut lhs: &[u16], mut rhs: &[u16]) -> Vec<u16> {
    const VECTOR_LENGTH: usize = mem::size_of::<u16x8>() / mem::size_of::<u16>();

    // we handle the degenerate cases
    if lhs.is_empty() {
        return Vec::new();
    } else if rhs.is_empty() {
        return lhs.to_vec();
    }

    let mut out = vec![0; lhs.len().max(rhs.len()) + 4096];
    let mut k = 0;

    // Why do we have to special case zero?
    if (lhs[0] == 0) || (rhs[0] == 0) {
        if (lhs[0] == 0) && (rhs[0] == 0) {
            lhs = &lhs[1..];
            rhs = &rhs[1..];
        } else if lhs[0] == 0 {
            out[k] = 0;
            k += 1;
            lhs = &lhs[1..];
        } else {
            rhs = &rhs[1..];
        }
    }

    let st_a = (lhs.len() / VECTOR_LENGTH) * VECTOR_LENGTH;
    let st_b = (rhs.len() / VECTOR_LENGTH) * VECTOR_LENGTH;

    let mut i = 0;
    let mut j = 0;
    if (i < st_a) && (j < st_b) {
        let mut v_a: u16x8 = Simd::from_slice(&lhs[i..]);
        let mut v_b: u16x8 = Simd::from_slice(&rhs[j..]);
        // we have a runningmask which indicates which values from A have been
        // spotted in B, these don't get written out.
        let mut runningmask_a_found_in_b: u8 = 0;
        loop {
            // a_found_in_b will contain a mask indicate for each entry in A
            // whether it is seen in B
            let a_found_in_b: u8 = matrix_cmp(v_a, v_b).to_bitmask()[0];
            runningmask_a_found_in_b |= a_found_in_b;
            // we always compare the last values of A and B
            let a_max: u16 = lhs[i + VECTOR_LENGTH - 1];
            let b_max: u16 = rhs[j + VECTOR_LENGTH - 1];
            if a_max <= b_max {
                // Ok. In this code path, we are ready to write our v_a
                // because there is no need to read more from B, they will
                // all be large values.
                let bitmask_belongs_to_difference = runningmask_a_found_in_b ^ 0xFF;
                let difference: u16x8 = unique_swizzle(v_a, bitmask_belongs_to_difference);
                store(difference, &mut out[k..]);
                k += bitmask_belongs_to_difference.count_ones() as usize;
                i += VECTOR_LENGTH;
                if i == st_a {
                    break;
                }
                runningmask_a_found_in_b = 0;
                v_a = Simd::from_slice(&lhs[i..]);
            }
            if b_max <= a_max {
                // in this code path, the current v_b has become useless
                j += VECTOR_LENGTH;
                if j == st_b {
                    break;
                }
                v_b = Simd::from_slice(&rhs[j..]);
            }
        }
        // End of main vectorized loop
        // At this point either i_a == st_a, which is the end of the vectorized processing,
        // or i_b == st_b and we are not done processing the vector...
        // so we need to finish it off.
        if i < st_a {
            let mut buffer: [u16; 8] = [0; 8]; // buffer to do a masked load
            buffer[..rhs.len() - j].copy_from_slice(&rhs[j..]);
            v_b = Simd::from_array(buffer);
            let a_found_in_b: u8 = matrix_cmp(v_a, v_b).to_bitmask()[0];
            runningmask_a_found_in_b |= a_found_in_b;
            let bitmask_belongs_to_difference: u8 = runningmask_a_found_in_b ^ 0xFF;
            let difference: u16x8 = unique_swizzle(v_a, bitmask_belongs_to_difference);
            store(difference, &mut out[k..]);
            k += bitmask_belongs_to_difference.count_ones() as usize;
            i += VECTOR_LENGTH;
        }
        // at this point we should have i_a == st_a and i_b == st_b
        // CRoaring comment says this is the case, but proptests panic if that's asserted
        // Is the comment wrong, or the code?
        //debug_assert_eq!(i, st_a);
        //debug_assert_eq!(j, st_b);
    }

    // do the tail using scalar code
    // TODO can this call out to some array util instead?
    while i < lhs.len() && j < rhs.len() {
        let a = lhs[i];
        let b = rhs[j];
        let cmp = a.cmp(&b);
        // match arms can be reordered, the ordering here is perf sensitive
        if cmp == Greater {
            j += 1;
        } else if cmp == Less {
            out[k] = a;
            k += 1;
            i += 1;
        } else {
            // cmp == Equal
            i += 1;
            j += 1;
        }
    }
    if i < lhs.len() {
        let n = lhs.len() - i;
        out[k..k + n].copy_from_slice(&lhs[i..i + n]);
        k += n;
    }

    out.truncate(k);
    out
}
