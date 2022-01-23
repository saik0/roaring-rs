use crate::bitmap::store::array::simd::lut::SHUFFLE_MASK;
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{matrix_cmp, store};
use std::mem;
use std::simd::{u16x8, u8x16, Simd};

/// Caller must ensure does not alias A or B
pub fn sub(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    const VECTOR_LENGTH: usize = mem::size_of::<u16x8>() / mem::size_of::<u16>();

    // we handle the degenerate case
    if lhs.is_empty() {
        return rhs.to_vec();
    }

    if rhs.is_empty() {
        return lhs.to_vec();
    }

    let st_a = (lhs.len() / VECTOR_LENGTH) * VECTOR_LENGTH;
    let st_b = (rhs.len() / VECTOR_LENGTH) * VECTOR_LENGTH;

    let mut out = vec![0; lhs.len().max(rhs.len()) + 4096];
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    if (i < st_a) && (j < st_b) {
        let mut v_a: u16x8 = Simd::from_slice(&lhs[i..]);
        let mut v_b: u16x8 = Simd::from_slice(&rhs[j..]);
        // we have a runningmask which indicates which values from A have been
        // spotted in B, these don't get written out.
        let mut runningmask_a_found_in_b: usize = 0;
        loop {
            // a_found_in_b will contain a mask indicate for each entry in A
            // whether it is seen in B
            let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a, v_b));
            runningmask_a_found_in_b |= a_found_in_b;
            // we always compare the last values of A and B
            let a_max: u16 = lhs[i + VECTOR_LENGTH - 1];
            let b_max: u16 = rhs[j + VECTOR_LENGTH - 1];
            if a_max <= b_max {
                // Ok. In this code path, we are ready to write our v_a
                // because there is no need to read more from B, they will
                // all be large values.
                let bitmask_belongs_to_difference = runningmask_a_found_in_b ^ 0xFF;
                /*** next few lines are probably expensive *****/
                // TODO aligned read?
                let sm16: u8x16 =
                    Simd::from_slice(&SHUFFLE_MASK[bitmask_belongs_to_difference * 16..]);
                // Safety: This is safe as the types are the same size
                // TODO make this a cast when it's supported
                let sm16: u16x8 = unsafe { mem::transmute(sm16) };
                let p: u16x8 = swizzle_u16x8(v_a, sm16);
                store(p, &mut out[k..]); // can overflow
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
        // at this point, either we have i_a == st_a, which is the end of the
        // vectorized processing,
        // or we have i_b == st_b,  and we are not done processing the vector...
        // so we need to finish it off.
        if i < st_a {
            // we have unfinished business...
            v_b = Simd::from_slice(&rhs[j..]);
            let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a, v_b));
            runningmask_a_found_in_b |= a_found_in_b;
            let bitmask_belongs_to_difference: usize = runningmask_a_found_in_b ^ 0xFF;
            let sm16: u8x16 = Simd::from_slice(&SHUFFLE_MASK[bitmask_belongs_to_difference * 16..]);
            // Safety: This is safe as the types are the same size
            // TODO make this a cast when it's supported
            let sm16: u16x8 = unsafe { mem::transmute(sm16) };
            let p: u16x8 = swizzle_u16x8(v_a, sm16);
            store(p, &mut out[k..]);
            k += bitmask_belongs_to_difference.count_ones() as usize;
            i += VECTOR_LENGTH;
        }
        // at this point we should have i_a == st_a and i_b == st_b
    }
    // do the tail using scalar code
    // match arms can be reordered, the ordering here is perf sensitive
    #[allow(clippy::comparison_chain)]
    while i < lhs.len() && j < rhs.len() {
        let a = lhs[i];
        let b = rhs[j];
        if a > b {
            j += 1;
        } else if a < b {
            out[k] = a;
            k += 1;
            i += 1;
        } else {
            i += 1;
            j += 1;
        }
    }

    // Why cant i_b be < rhs.len()?
    if i < lhs.len() {
        let n = lhs.len() - i;
        out[k..k + n].copy_from_slice(&lhs[i..i + n]);
        k += n;
    }
    out.truncate(k);
    out
}
