use crate::bitmap::store::array::{and_array_walk, ArrayBinaryOperationVisitor};
use core_simd::u16x8;

use crate::bitmap::store::array::simd::{load, matrix_cmp};

// Ported from CRoaring and arXiv:1709.07821
// Lemire et al, Roaring Bitmaps: Implementation of an Optimized Software Library
// Prior work: Schlegel et al., Fast Sorted-Set Intersection using SIMD Instructions
//
// Rust port notes:
// The x86 PCMPESTRM instruction been replaced with a simple SIMD or-shift
// While several more instructions, this is what is available through LLVM intrinsics
// and is portable to other ISAs. The performance is comparable on x86.
pub fn and(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    let st_a = (lhs.len() / u16x8::LANES) * u16x8::LANES;
    let st_b = (rhs.len() / u16x8::LANES) * u16x8::LANES;

    let mut i: usize = 0;
    let mut j: usize = 0;
    if (i < st_a) && (j < st_b) {
        let mut v_a: u16x8 = load(&lhs[i..]);
        let mut v_b: u16x8 = load(&rhs[j..]);
        loop {
            let mask = matrix_cmp(v_a, v_b).to_bitmask()[0];
            visitor.visit_vector(v_a, mask);

            let a_max: u16 = lhs[i + u16x8::LANES - 1];
            let b_max: u16 = rhs[j + u16x8::LANES - 1];
            if a_max <= b_max {
                i += u16x8::LANES;
                if i == st_a {
                    break;
                }
                v_a = load(&lhs[i..]);
            }
            if b_max <= a_max {
                j += u16x8::LANES;
                if j == st_b {
                    break;
                }
                v_b = load(&rhs[j..]);
            }
        }
    }

    // intersect the tail using scalar intersection
    and_array_walk(&lhs[i..], &rhs[j..], visitor);
}
