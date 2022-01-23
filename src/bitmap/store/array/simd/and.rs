use crate::bitmap::store::array::simd::lut::SHUFFLE_MASK;
use crate::simd::compat::{swizzle_u16x8, to_bitmask};
use crate::simd::util::{load_unchecked, matrix_cmp, store_unchecked};
use mem::transmute;
use std::mem;
use std::simd::{u16x8, u8x16};

// From Schlegel et al., Fast Sorted-Set Intersection using SIMD Instructions
//
// Impl note:
// The x86 PCMPESTRM used in the paper has been replaced with a SIMD or-shift
// While several more instructions, it is portable to many SIMD ISAs
// Benchmarked on my hardware: the runtime was within 1 std dev of D. Lemire's impl in CRoaring
pub fn and(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    const VEC_LEN: usize = mem::size_of::<u16x8>() / mem::size_of::<u16>();

    let st_a = (lhs.len() / VEC_LEN) * VEC_LEN;
    let st_b = (rhs.len() / VEC_LEN) * VEC_LEN;

    let mut out = vec![0; lhs.len().max(rhs.len())];

    let mut i: usize = 0;
    let mut j: usize = 0;
    let mut k: usize = 0;
    if (i < st_a) && (j < st_b) {
        // Safety:
        //  * Unchecked loads fom lhs[i..] and rhs[j..] are safe given i < st_a && j < st_b
        let mut v_a: u16x8 = unsafe { load_unchecked(&lhs[i..]) };
        let mut v_b: u16x8 = unsafe { load_unchecked(&rhs[j..]) };
        loop {
            let r = to_bitmask(matrix_cmp(v_a, v_b));

            // Safety:
            //  * r is guaranteed to be 1 byte at most.
            //    256 * 16 == 4096, which is the len of SHUFFLE_MASK
            let key: u8x16 = unsafe { load_unchecked(&SHUFFLE_MASK[r * 16..]) };

            // Safety:
            //  * These types are the same size.
            //  * TODO replace with cast once it's in nightly
            let key: u16x8 = unsafe { transmute(key) };
            let intersection: u16x8 = swizzle_u16x8(v_a, key);

            // Safety:
            //  * Unchecked store to out[k..] k is always <= i or j
            unsafe { store_unchecked(intersection, &mut out[k..]) };
            k += r.count_ones() as usize;

            // Safety:
            //  * Must be in bounds given i < st_a && j < st_b checks
            let a_max: u16 = unsafe { *lhs.get_unchecked(i + VEC_LEN - 1) };
            let b_max: u16 = unsafe { *rhs.get_unchecked(j + VEC_LEN - 1) };
            if a_max <= b_max {
                i += VEC_LEN;
                if i == st_a {
                    break;
                }
                // Safety:
                //  * Unchecked loads fom lhs[i..] is save given i != st_a
                v_a = unsafe { load_unchecked(&lhs[i..]) };
            }
            if b_max <= a_max {
                j += VEC_LEN;
                if j == st_b {
                    break;
                }
                // Safety:
                //  * Unchecked loads fom rhs[j..] is save given j != st_b
                v_b = unsafe { load_unchecked(&rhs[j..]) };
            }
        }
    }

    // intersect the tail using scalar intersection
    // TODO finish up by calling normal scalar walk/run fn instead this inlined walk?
    // Safety:
    //  * Unchecked indexing safe given the condition of the loop

    while i < lhs.len() && j < rhs.len() {
        let a: u16 = unsafe { *lhs.get_unchecked(i) };
        let b: u16 = unsafe { *rhs.get_unchecked(j) };

        // Match arms can be reordered and this is a performance sensitive loop
        #[allow(clippy::comparison_chain)]
        if a < b {
            i += 1;
        } else if a > b {
            j += 1;
        } else {
            *unsafe { out.get_unchecked_mut(k) } = a; //==b;
            k += 1;
            i += 1;
            j += 1;
        }
    }

    out.truncate(k);
    out
}
