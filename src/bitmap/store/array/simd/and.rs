use core_simd::u16x8;

use crate::bitmap::store::array::simd::{load, matrix_cmp};
use crate::bitmap::store::array_store::visitor::ArrayBinaryOperationVisitor;
use std::mem;

// From Schlegel et al., Fast Sorted-Set Intersection using SIMD Instructions
//
// Impl note:
// The x86 PCMPESTRM used in the paper has been replaced with a SIMD or-shift
// While several more instructions, it is portable to many SIMD ISAs
// Benchmarked on my hardware: the run time was comparable
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

            // Safety:
            //  * Must be in bounds given i < st_a && j < st_b checks
            // let a_max: u16 = lhs[i + VEC_LEN - 1];
            // let b_max: u16 = rhs[j + VEC_LEN - 1];
            let a_max: u16 = unsafe { *lhs.get_unchecked(i + u16x8::LANES - 1) };
            let b_max: u16 = unsafe { *rhs.get_unchecked(j + u16x8::LANES - 1) };
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
    and_assign_walk(&lhs[i..], &rhs[j..], visitor);
}

#[inline]
fn and_assign_walk(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    use std::cmp::Ordering;

    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Ordering::Less => {
                i += 1;
            }
            Ordering::Greater => {
                j += 1;
            }
            Ordering::Equal => {
                visitor.visit_scalar(*a);
                i += 1;
                j += 1;
            }
        }
    }
}
