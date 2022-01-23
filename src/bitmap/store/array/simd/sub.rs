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

    let lhs_len = lhs.len();
    let rhs_len = rhs.len();

    let mut out = vec![0; lhs.len().max(rhs.len()) + 4096];
    let mut count = 0;
    let mut i_a = 0;
    let mut i_b = 0;
    let st_a = (lhs_len / VECTOR_LENGTH) * VECTOR_LENGTH;
    let st_b = (rhs_len / VECTOR_LENGTH) * VECTOR_LENGTH;

    if (i_a < st_a) && (i_b < st_b) {
        // this is the vectorized code path

        //, v_bmax;
        // we load a vector from A and a vector from B
        let mut v_a: u16x8 = Simd::from_slice(&lhs[i_a..]);
        let mut v_b: u16x8 = Simd::from_slice(&rhs[i_b..]);
        // we have a runningmask which indicates which values from A have been
        // spotted in B, these don't get written out.
        let mut runningmask_a_found_in_b: usize = 0;
        /****
         * start of the main vectorized loop
         *****/
        loop {
            // a_found_in_b will contain a mask indicate for each entry in A
            // whether it is seen in B
            let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a, v_b));
            runningmask_a_found_in_b |= a_found_in_b;
            // we always compare the last values of A and B
            let a_max: u16 = lhs[i_a + VECTOR_LENGTH - 1];
            let b_max: u16 = rhs[i_b + VECTOR_LENGTH - 1];
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
                store(p, &mut out[count..]); // can overflow
                count += bitmask_belongs_to_difference.count_ones() as usize;
                // we advance a
                i_a += VECTOR_LENGTH;
                if i_a == st_a {
                    // no more
                    break;
                }
                runningmask_a_found_in_b = 0;
                v_a = Simd::from_slice(&lhs[i_a..]);
            }
            if b_max <= a_max {
                // in this code path, the current v_b has become useless
                i_b += VECTOR_LENGTH;
                if i_b == st_b {
                    break;
                }
                v_b = Simd::from_slice(&rhs[i_b..]);
            }
        }
        // at this point, either we have i_a == st_a, which is the end of the
        // vectorized processing,
        // or we have i_b == st_b,  and we are not done processing the vector...
        // so we need to finish it off.
        if i_a < st_a {
            // we have unfinished business...
            let mut buffer: [u16; 8] = [0; 8]; // buffer to do a masked load
            buffer[..rhs_len - i_b].copy_from_slice(&rhs[i_b..]);
            v_b = Simd::from_array(buffer);
            let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a, v_b));
            runningmask_a_found_in_b |= a_found_in_b;
            let bitmask_belongs_to_difference: usize = runningmask_a_found_in_b ^ 0xFF;
            let sm16: u8x16 = Simd::from_slice(&SHUFFLE_MASK[bitmask_belongs_to_difference * 16..]);
            // Safety: This is safe as the types are the same size
            // TODO make this a cast when it's supported
            let sm16: u16x8 = unsafe { mem::transmute(sm16) };
            let p: u16x8 = swizzle_u16x8(v_a, sm16);
            store(p, &mut out[count..]);
            count += bitmask_belongs_to_difference.count_ones() as usize;
            i_a += VECTOR_LENGTH;
        }
        // at this point we should have i_a == st_a and i_b == st_b
    }
    // do the tail using scalar code
    // match arms can be reordered, the ordering here is perf sensitive
    #[allow(clippy::comparison_chain)]
    while i_a < lhs_len && i_b < rhs_len {
        let a = lhs[i_a];
        let b = rhs[i_b];
        if a > b {
            i_b += 1;
        } else if a < b {
            out[count] = a;
            count += 1;
            i_a += 1;
        } else {
            i_a += 1;
            i_b += 1;
        }
    }

    // Why cant i_b be < rhs_len?
    if i_a < lhs_len {
        let n = lhs_len - i_a;
        out[count..n + count].copy_from_slice(&lhs[i_a..n + i_a]);
        count += n;
    }
    out.truncate(count);
    out
}
