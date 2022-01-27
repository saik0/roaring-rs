use crate::bitmap::store::array::simd::{load, matrix_cmp, store, unique_swizzle};
use crate::bitmap::store::array::sub_walk_mut;
use crate::bitmap::store::array_store::visitor::ArrayBinaryOperationVisitor;
use core_simd::{u16x8, Simd};
use std::mem;

pub fn sub(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    // we handle the degenerate cases
    if lhs.is_empty() {
        return;
    } else if rhs.is_empty() {
        visitor.visit_slice(lhs);
        return;
    }

    let st_a = (lhs.len() / u16x8::LANES) * u16x8::LANES;
    let st_b = (rhs.len() / u16x8::LANES) * u16x8::LANES;

    let mut i = 0;
    let mut j = 0;
    if (i < st_a) && (j < st_b) {
        let mut v_a: u16x8 = load(&lhs[i..]);
        let mut v_b: u16x8 = load(&rhs[j..]);
        // we have a runningmask which indicates which values from A have been
        // spotted in B, these don't get written out.
        let mut runningmask_a_found_in_b: u8 = 0;
        loop {
            // a_found_in_b will contain a mask indicate for each entry in A
            // whether it is seen in B
            let a_found_in_b: u8 = matrix_cmp(v_a, v_b).to_bitmask()[0];
            runningmask_a_found_in_b |= a_found_in_b;
            // we always compare the last values of A and B
            let a_max: u16 = lhs[i + u16x8::LANES - 1];
            let b_max: u16 = rhs[j + u16x8::LANES - 1];
            if a_max <= b_max {
                // Ok. In this code path, we are ready to write our v_a
                // because there is no need to read more from B, they will
                // all be large values.
                let bitmask_belongs_to_difference = runningmask_a_found_in_b ^ 0xFF;
                visitor.visit_vector(v_a, bitmask_belongs_to_difference);
                i += u16x8::LANES;
                if i == st_a {
                    break;
                }
                runningmask_a_found_in_b = 0;
                v_a = load(&lhs[i..]);
            }
            if b_max <= a_max {
                // in this code path, the current v_b has become useless
                j += u16x8::LANES;
                if j == st_b {
                    break;
                }
                v_b = load(&rhs[j..]);
            }
        }

        debug_assert!(i == st_a || j == st_b);

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
            visitor.visit_vector(v_a, bitmask_belongs_to_difference);
            i += u16x8::LANES;
        }
    }

    // do the tail using scalar code
    sub_walk(&lhs[i..], &rhs[j..], visitor);
}

fn sub_walk(lhs: &[u16], rhs: &[u16], visitor: &mut impl ArrayBinaryOperationVisitor) {
    use std::cmp::Ordering;
    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Ordering::Less => {
                visitor.visit_scalar(*a);
                i += 1;
            }
            Ordering::Greater => j += 1,
            Ordering::Equal => {
                i += 1;
                j += 1;
            }
        }
    }

    // Store remaining elements of the left array
    visitor.visit_slice(&lhs[i..]);
}
