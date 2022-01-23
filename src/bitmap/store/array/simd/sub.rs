use crate::bitmap::store::array::simd::lut::SHUFFLE_MASK;
use crate::simd::compat::to_bitmask;
use crate::simd::util::matrix_cmp;

// A shim until we rewrite method args / return type
pub fn sub(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut out = vec![0; lhs.len().max(rhs.len()) + 4096];
    let len = unsafe { _difference_vector_x86(lhs, rhs, out.as_mut_ptr()) };
    out.truncate(len);
    out
}

/// Caller must ensure does not alias A or B
unsafe fn _difference_vector_x86(mut lhs: &[u16], mut rhs: &[u16], out: *mut u16) -> usize {
    use std::arch::x86_64::{
        __m128i, _mm_lddqu_si128, _mm_load_si128, _mm_loadu_si128, _mm_shuffle_epi8,
        _mm_storeu_si128,
    };
    use std::mem;
    use std::ptr::{copy_nonoverlapping, write_bytes};
    const VECTOR_LENGTH: usize = mem::size_of::<__m128i>() / mem::size_of::<u16>();

    let mut lhs_len = lhs.len();
    let mut rhs_len = rhs.len();

    unsafe {
        // we handle the degenerate case
        if lhs_len == 0 {
            copy_nonoverlapping(rhs.as_ptr(), out, rhs_len);
            return rhs_len;
        }

        if rhs_len == 0 {
            copy_nonoverlapping(lhs.as_ptr(), out, lhs_len);
            return lhs_len;
        }
        // handle the leading zeroes, it is messy but it allows us to use the fast
        // _mm_cmpistrm instrinsic safely
        let mut count = 0;
        if (lhs[0] == 0) || (rhs[0] == 0) {
            if (lhs[0] == 0) && (rhs[0] == 0) {
                lhs = &lhs[1..];
                lhs_len -= 1;
                rhs = &rhs[1..];
                rhs_len -= 1;
            } else if lhs[0] == 0 {
                *out.add(count) = 0;
                count += 1;
                lhs = &lhs[1..];
                lhs_len -= 1;
            } else {
                rhs = &rhs[1..];
                rhs_len -= 1;
            }
        }
        // at this point, we have two non-empty arrays, made of non-zero
        // increasing values.
        let mut i_a = 0;
        let mut i_b = 0;

        let st_a = (lhs_len / VECTOR_LENGTH) * VECTOR_LENGTH;
        let st_b = (rhs_len / VECTOR_LENGTH) * VECTOR_LENGTH;

        if (i_a < st_a) && (i_b < st_b) {
            // this is the vectorized code path

            //, v_bmax;
            // we load a vector from A and a vector from B
            // v_a = _mm_lddqu_si128((__m128i *)&A[i_a]);
            // v_b = _mm_lddqu_si128((__m128i *)&B[i_b]);
            let mut v_a: __m128i = _mm_lddqu_si128(lhs.as_ptr().add(i_a).cast::<__m128i>());
            let mut v_b: __m128i = _mm_lddqu_si128(rhs.as_ptr().add(i_b).cast::<__m128i>());
            // we have a runningmask which indicates which values from A have been
            // spotted in B, these don't get written out.
            let mut runningmask_a_found_in_b: usize = 0;
            /****
             * start of the main vectorized loop
             *****/
            loop {
                // afoundinb will contain a mask indicate for each entry in A
                // whether it is seen
                // in B
                let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a.into(), v_b.into()));
                runningmask_a_found_in_b |= a_found_in_b;
                // we always compare the last values of A and B
                // const uint16_t a_max = A[i_a + vectorlength - 1];
                // const uint16_t b_max = B[i_b + vectorlength - 1];
                let a_max: u16 = lhs[i_a + VECTOR_LENGTH - 1];
                let b_max: u16 = rhs[i_b + VECTOR_LENGTH - 1];
                if a_max <= b_max {
                    // Ok. In this code path, we are ready to write our v_a
                    // because there is no need to read more from B, they will
                    // all be large values.
                    let bitmask_belongs_to_difference = runningmask_a_found_in_b ^ 0xFF;
                    /*** next few lines are probably expensive *****/
                    // TODO aligned
                    let sm16: __m128i = _mm_loadu_si128(
                        SHUFFLE_MASK.as_ptr().cast::<__m128i>().add(bitmask_belongs_to_difference),
                    );
                    let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
                    _mm_storeu_si128(out.add(count).cast::<__m128i>(), p); // can overflow
                    count += bitmask_belongs_to_difference.count_ones() as usize;
                    // we advance a
                    i_a += VECTOR_LENGTH;
                    if i_a == st_a {
                        // no more
                        break;
                    }
                    runningmask_a_found_in_b = 0;
                    // v_a = _mm_lddqu_si128((__m128i *)&A[i_a]);
                    v_a = _mm_lddqu_si128(lhs.as_ptr().add(i_a).cast::<__m128i>());
                }
                if b_max <= a_max {
                    // in this code path, the current v_b has become useless
                    i_b += VECTOR_LENGTH;
                    if i_b == st_b {
                        break;
                    }
                    //v_b = _mm_lddqu_si128((__m128i *)&B[i_b]);
                    v_b = _mm_lddqu_si128(rhs.as_ptr().add(i_b).cast::<__m128i>());
                }
            }
            // at this point, either we have i_a == st_a, which is the end of the
            // vectorized processing,
            // or we have i_b == st_b,  and we are not done processing the vector...
            // so we need to finish it off.
            if i_a < st_a {
                // we have unfinished business...
                let mut buffer: [u16; 8] = [0; 8]; // buffer to do a masked load
                write_bytes(buffer.as_mut_ptr(), 0, 8);
                copy_nonoverlapping(rhs.as_ptr().add(i_b), buffer.as_mut_ptr(), rhs_len - i_b);
                v_b = _mm_lddqu_si128(buffer.as_ptr().cast());
                let a_found_in_b: usize = to_bitmask(matrix_cmp(v_a.into(), v_b.into()));
                runningmask_a_found_in_b |= a_found_in_b;
                let bitmask_belongs_to_difference: usize = runningmask_a_found_in_b ^ 0xFF;
                let sm16: __m128i = _mm_load_si128(
                    SHUFFLE_MASK.as_ptr().cast::<__m128i>().add(bitmask_belongs_to_difference),
                );
                let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
                _mm_storeu_si128(out.add(count).cast::<__m128i>(), p); // can overflow
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
                *out.add(count) = a;
                count += 1;
                i_a += 1;
            } else {
                //==
                i_a += 1;
                i_b += 1;
            }
        }
        if i_a < lhs_len {
            if out as *const u16 == lhs.as_ptr() {
                assert!(count <= i_a);
                if count < i_a {
                    copy_nonoverlapping(lhs.as_ptr().add(i_a), out.add(count), lhs_len - i_a);
                }
            } else {
                for i in 0..(lhs_len - i_a) {
                    *out.add(count + i) = lhs[i + i_a];
                }
            }
            count += lhs_len - i_a;
        }
        count
    }
}
