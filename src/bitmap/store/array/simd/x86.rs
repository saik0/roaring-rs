// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██╗░░██╗░█████╗░░█████╗░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░╚██╗██╔╝██╔══██╗██╔═══╝░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░╚███╔╝░╚█████╔╝██████╗░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██╔██╗░██╔══██╗██╔══██╗░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░██╔╝╚██╗╚█████╔╝╚█████╔╝░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░╚═╝░░╚═╝░╚════╝░░╚════╝░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
// ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
//
// x86 SIMD intrinsics included temporarily for ease of comparing benchmarks

use crate::bitmap::store::array::simd::lut::{uniqshuf, SHUFFLE_MASK};
use std::arch::x86_64::{
    __m128i, _mm_alignr_epi8, _mm_cmpeq_epi16, _mm_cmpestrm, _mm_cmpistrm, _mm_extract_epi32,
    _mm_lddqu_si128, _mm_load_si128, _mm_loadu_si128, _mm_max_epu16, _mm_min_epu16,
    _mm_movemask_epi8, _mm_or_si128, _mm_packs_epi16, _mm_set1_epi16, _mm_setzero_si128,
    _mm_shuffle_epi8, _mm_storeu_si128, _popcnt32, _SIDD_BIT_MASK, _SIDD_CMP_EQUAL_ANY,
    _SIDD_UWORD_OPS,
};
use std::mem;
use std::ptr::{copy_nonoverlapping, write_bytes};

// Safe entry points

#[inline]
pub fn or_x86_simd(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut vec = Vec::with_capacity(lhs.len() + rhs.len());
    unsafe {
        let len =
            union_vector16_x86(lhs.as_ptr(), lhs.len(), rhs.as_ptr(), rhs.len(), vec.as_mut_ptr());
        vec.set_len(len);
    }
    vec
}

#[inline]
pub fn and_x86_simd(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut vec = Vec::with_capacity(lhs.len().max(rhs.len()));
    unsafe {
        let len = intersect_vector16_x86(
            lhs.as_ptr(),
            lhs.len(),
            rhs.as_ptr(),
            rhs.len(),
            vec.as_mut_ptr(),
        );
        vec.set_len(len);
    }
    vec
}

#[inline]
pub fn and_assign_x86_simd(lhs: &[u16], rhs: &[u16], buf: &mut Vec<u16>) {
    let min_capacity = lhs.len().max(rhs.len());
    if min_capacity > buf.len() {
        buf.reserve(min_capacity - buf.len())
    }
    unsafe {
        let len = intersect_vector16_x86(
            lhs.as_ptr(),
            lhs.len(),
            rhs.as_ptr(),
            rhs.len(),
            buf.as_mut_ptr(),
        );
        buf.set_len(len);
    }
}

pub fn sub_x86_simd(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut out = vec![0; lhs.len().max(rhs.len()) + 4096];
    let len = unsafe {
        _difference_vector_x86(lhs.as_ptr(), lhs.len(), rhs.as_ptr(), rhs.len(), out.as_mut_ptr())
    };
    out.truncate(len);
    out
}

// Here be dragons!

const CMPESTRM_CTRL: i32 = _SIDD_UWORD_OPS | _SIDD_CMP_EQUAL_ANY | _SIDD_BIT_MASK;

/**
 * From Schlegel et al., Fast Sorted-Set Intersection using SIMD Instructions
 * Optimized by D. Lemire on May 3rd 2013
 */
#[allow(non_snake_case)]
unsafe fn intersect_vector16_x86(
    V_A: *const u16,
    s_a: usize,
    V_B: *const u16,
    s_b: usize,
    C: *mut u16,
) -> usize {
    let mut count: usize = 0;
    let mut i_a: usize = 0;
    let mut i_b: usize = 0;
    let vectorlength: usize = mem::size_of::<__m128i>() / mem::size_of::<u16>();
    let st_a = (s_a / vectorlength) * vectorlength;
    let st_b = (s_b / vectorlength) * vectorlength;
    let mut v_a: __m128i;
    let mut v_b: __m128i;
    if (i_a < st_a) && (i_b < st_b) {
        // v_a = _mm_lddqu_si128((__m128i *)&A[i_a]);
        // v_b = _mm_lddqu_si128((__m128i *)&B[i_b]);
        v_a = _mm_lddqu_si128(V_A.add(i_a).cast::<__m128i>());
        v_b = _mm_lddqu_si128(V_B.add(i_b).cast::<__m128i>());
        while (*V_A.add(i_a) == 0) || (*V_B.add(i_b) == 0) {
            let res_v: __m128i =
                _mm_cmpestrm::<CMPESTRM_CTRL>(v_b, vectorlength as i32, v_a, vectorlength as i32);
            let r: i32 = _mm_extract_epi32::<0>(res_v);
            // TODO THIS SHOULD BE ALIGNED
            //let sm16: __m128i = _mm_load_si128(shuffle_mask16.as_ptr().offset(r as isize).cast::<__m128i>());
            let sm16: __m128i =
                _mm_loadu_si128(SHUFFLE_MASK.as_ptr().cast::<__m128i>().offset(r as isize));
            let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
            _mm_storeu_si128(C.add(count).cast::<__m128i>(), p); // can overflow
            count += _popcnt32(r) as usize;
            let a_max: u16 = *V_A.add(i_a + vectorlength - 1);
            let b_max: u16 = *V_B.add(i_b + vectorlength - 1);
            if a_max <= b_max {
                i_a += vectorlength;
                if i_a == st_a {
                    break;
                }
                v_a = _mm_lddqu_si128(V_A.add(i_a).cast::<__m128i>());
            }
            if b_max <= a_max {
                i_b += vectorlength;
                if i_b == st_b {
                    break;
                }
                v_b = _mm_lddqu_si128(V_B.add(i_b).cast::<__m128i>());
            }
        }
        if (i_a < st_a) && (i_b < st_b) {
            loop {
                let res_v: __m128i = _mm_cmpistrm::<CMPESTRM_CTRL>(v_b, v_a);
                let r: i32 = _mm_extract_epi32::<0>(res_v);
                // TODO THIS SHOULD BE ALIGNED
                //let sm16: __m128i = _mm_load_si128(shuffle_mask16.as_ptr().offset(r as isize).cast::<__m128i>());
                let sm16: __m128i =
                    _mm_loadu_si128(SHUFFLE_MASK.as_ptr().cast::<__m128i>().offset(r as isize));
                let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
                _mm_storeu_si128(C.add(count).cast::<__m128i>(), p); // can overflow
                count += _popcnt32(r) as usize;
                let a_max: u16 = *V_A.add(i_a + vectorlength - 1);
                let b_max: u16 = *V_B.add(i_b + vectorlength - 1);
                if a_max <= b_max {
                    i_a += vectorlength;
                    if i_a == st_a {
                        break;
                    }
                    v_a = _mm_lddqu_si128(V_A.add(i_a).cast::<__m128i>());
                }
                if b_max <= a_max {
                    i_b += vectorlength;
                    if i_b == st_b {
                        break;
                    }
                    v_b = _mm_lddqu_si128(V_B.add(i_b).cast::<__m128i>());
                }
            }
        }
    }
    // intersect the tail using scalar intersection
    while i_a < s_a && i_b < s_b {
        let a: u16 = *V_A.add(i_a);
        let b: u16 = *V_B.add(i_b);
        if a < b {
            i_a += 1;
        } else if b < a {
            i_b += 1;
        } else {
            *C.add(count) = a; //==b;
            count += 1;
            i_a += 1;
            i_b += 1;
        }
    }
    count as usize
}

// can one vectorize the computation of the union? (Update: Yes! See
// union_vector16).
#[allow(non_snake_case)]
unsafe fn union_uint16_x86(
    set_1: *const u16,
    size_1: usize,
    set_2: *const u16,
    size_2: usize,
    buffer: *mut u16,
) -> usize {
    let mut pos = 0;
    let mut idx_1 = 0;
    let mut idx_2 = 0;

    // Translation notes: changed copy to copy_nonoverlapping
    // buffer != set_1 && buffer != set_2

    if 0 == size_2 {
        copy_nonoverlapping(set_1, buffer, size_1);
        return size_1;
    }
    if 0 == size_1 {
        copy_nonoverlapping(set_2, buffer, size_2);
        return size_2;
    }

    let mut val_1: u16 = *set_1;
    let mut val_2: u16 = *set_2;

    loop {
        if val_1 < val_2 {
            *buffer.add(pos) = val_1;
            pos += 1;
            idx_1 += 1;
            if idx_1 >= size_1 {
                break;
            }
            val_1 = *set_1.add(idx_1);
        } else if val_2 < val_1 {
            *buffer.add(pos) = val_2;
            pos += 1;
            idx_2 += 1;
            if idx_2 >= size_2 {
                break;
            }
            val_2 = *set_2.add(idx_2);
        } else {
            *buffer.add(pos) = val_1;
            pos += 1;
            idx_1 += 1;
            idx_2 += 1;
            if idx_1 >= size_1 || idx_2 >= size_2 {
                break;
            }
            val_1 = *set_1.add(idx_1);
            val_2 = *set_2.add(idx_2);
        }
    }

    if idx_1 < size_1 {
        let n_elems = size_1 - idx_1;
        copy_nonoverlapping(set_1.add(idx_1), buffer.add(pos), n_elems);
        pos += n_elems;
    } else if idx_2 < size_2 {
        let n_elems = size_2 - idx_2;
        copy_nonoverlapping(set_2.add(idx_2), buffer.add(pos), n_elems);
        pos += n_elems;
    }

    pos
}

/***
 * start of the SIMD 16-bit union code
 *
 */

// Assuming that vInput1 and vInput2 are sorted, produces a sorted output going
// from vecMin all the way to vecMax
// developed originally for merge sort using SIMD instructions.
// Standard merge. See, e.g., Inoue and Taura, SIMD- and Cache-Friendly
// Algorithm for Sorting an Array of Structures
#[allow(non_snake_case)]
#[inline]
unsafe fn sse_merge_x86(
    vInput1: *const __m128i,
    vInput2: *const __m128i, // input 1 & 2
    vecMin: *mut __m128i,
    vecMax: *mut __m128i, // output
) {
    let mut vecTmp: __m128i = _mm_min_epu16(*vInput1, *vInput2);
    *vecMax = _mm_max_epu16(*vInput1, *vInput2);
    vecTmp = _mm_alignr_epi8::<2>(vecTmp, vecTmp);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    vecTmp = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
    *vecMin = _mm_min_epu16(vecTmp, *vecMax);
    *vecMax = _mm_max_epu16(vecTmp, *vecMax);
    *vecMin = _mm_alignr_epi8::<2>(*vecMin, *vecMin);
}

// write vector new, while omitting repeated values assuming that previously
// written vector was "old"
#[allow(non_snake_case)]
#[inline]
unsafe fn store_unique_x86(old: __m128i, newval: __m128i, output: *mut u16) -> usize {
    let vecTmp: __m128i = _mm_alignr_epi8::<14>(newval, old);
    // lots of high latency instructions follow (optimize?)
    let M: i32 =
        _mm_movemask_epi8(_mm_packs_epi16(_mm_cmpeq_epi16(vecTmp, newval), _mm_setzero_si128()));
    let numberofnewvalues: usize = 8 - _popcnt32(M) as usize;
    let key: __m128i = _mm_lddqu_si128(uniqshuf.as_ptr().cast::<__m128i>().offset(M as isize));
    let val: __m128i = _mm_shuffle_epi8(newval, key);
    _mm_storeu_si128(output as *mut __m128i, val);
    numberofnewvalues
}

// working in-place, this function overwrites the repeated values
// could be avoided?
#[allow(non_snake_case)]
#[inline]
unsafe fn unique_x86(out: *mut u16, len: usize) -> usize {
    let mut pos: usize = 1;
    for i in 1..len {
        if *out.add(i) != *out.add(i - 1) {
            *out.add(pos) = *out.add(i);
            pos += 1;
        }
    }
    pos
}

// a one-pass SSE union algorithm
// This function may not be safe if array1 == output or array2 == output.
#[allow(non_camel_case_types, non_snake_case)]
unsafe fn union_vector16_x86(
    array1: *const u16,
    length1: usize,
    array2: *const u16,
    length2: usize,
    mut output: *mut u16,
) -> usize {
    if (length1 < 8) || (length2 < 8) {
        return union_uint16_x86(array1, length1, array2, length2, output);
    }

    let mut V: __m128i;
    let mut vecMin: __m128i = _mm_setzero_si128();
    let mut vecMax: __m128i = _mm_setzero_si128();

    let initoutput: *mut u16 = output;
    let len1: usize = length1 / 8;
    let len2: usize = length2 / 8;
    let mut pos1: usize = 0;
    let mut pos2: usize = 0;

    // we start the machine
    let vA: __m128i = _mm_lddqu_si128(array1.cast());
    pos1 += 1;
    let vB: __m128i = _mm_lddqu_si128(array2.cast());
    pos2 += 1;

    sse_merge_x86(&vA, &vB, &mut vecMin, &mut vecMax);
    let mut laststore: __m128i = _mm_set1_epi16(-1);
    output = output.add(store_unique_x86(laststore, vecMin, output));
    laststore = vecMin;
    if (pos1 < len1) && (pos2 < len2) {
        let mut curA: u16 = *array1.add(8 * pos1);
        let mut curB: u16 = *array2.add(8 * pos2);
        loop {
            if curA <= curB {
                V = _mm_lddqu_si128((array1).cast::<__m128i>().add(pos1));
                pos1 += 1;
                if pos1 < len1 {
                    curA = *array1.add(8 * pos1);
                } else {
                    break;
                }
            } else {
                V = _mm_lddqu_si128((array2).cast::<__m128i>().add(pos2));
                pos2 += 1;
                if pos2 < len2 {
                    curB = *array2.add(8 * pos2);
                } else {
                    break;
                }
            }
            sse_merge_x86(&V, &vecMax, &mut vecMin, &mut vecMax);
            output = output.add(store_unique_x86(laststore, vecMin, output));
            laststore = vecMin;
        }
        sse_merge_x86(&V, &vecMax, &mut vecMin, &mut vecMax);
        output = output.add(store_unique_x86(laststore, vecMin, output));
        laststore = vecMin;
    }
    // we finish the rest off using a scalar algorithm
    // could be improved?
    //
    // copy the small end on a tmp buffer
    let mut len: usize = (output.offset_from(initoutput)) as usize;
    let mut buffer: [u16; 16] = [0; 16];
    let mut leftoversize = store_unique_x86(laststore, vecMax, buffer.as_mut_ptr());
    if pos1 == len1 {
        copy_nonoverlapping(
            array1.add(8 * pos1),
            buffer.as_mut_ptr().add(leftoversize),
            length1 - 8 * len1,
        );
        leftoversize += length1 - 8 * len1;
        buffer[..leftoversize as usize].sort_unstable();
        leftoversize = unique_x86(buffer.as_mut_ptr(), leftoversize);
        len += union_uint16_x86(
            buffer.as_mut_ptr(),
            leftoversize,
            array2.add(8 * pos2),
            length2 - 8 * pos2,
            output,
        );
    } else {
        copy_nonoverlapping(
            array2.add(8 * pos2),
            buffer.as_mut_ptr().add(leftoversize),
            length2 - 8 * len2,
        );
        leftoversize += length2 - 8 * len2;
        buffer[..leftoversize as usize].sort_unstable();
        leftoversize = unique_x86(buffer.as_mut_ptr(), leftoversize);
        len += union_uint16_x86(
            buffer.as_mut_ptr(),
            leftoversize,
            array1.add(8 * pos1),
            length1 - 8 * pos1,
            output,
        );
    }
    len
}

/// Caller must ensure does not alias A or B
unsafe fn _difference_vector_x86(
    mut A: *const u16,
    mut s_a: usize,
    mut B: *const u16,
    mut s_b: usize,
    C: *mut u16,
) -> usize {
    // we handle the degenerate case
    if s_a == 0 {
        copy_nonoverlapping(B, C, s_b);
        return s_b;
    }

    if s_b == 0 {
        copy_nonoverlapping(A, C, s_a);
        return s_a;
    }
    // handle the leading zeroes, it is messy but it allows us to use the fast
    // _mm_cmpistrm instrinsic safely
    let mut count = 0;
    if (*A == 0) || (*B == 0) {
        if (*A == 0) && (*B == 0) {
            A = A.offset(1);
            s_a -= 1;
            B = B.offset(1);
            s_b -= 1;
        } else if *A == 0 {
            *C.add(count) = 0;
            count += 1;
            A = A.offset(1);
            s_a -= 1;
        } else {
            B = B.offset(1);
            s_b -= 1;
        }
    }
    // at this point, we have two non-empty arrays, made of non-zero
    // increasing values.
    let mut i_a = 0;
    let mut i_b = 0;
    const vectorlength: usize = mem::size_of::<__m128i>() / mem::size_of::<u16>();
    let st_a = (s_a / vectorlength) * vectorlength;
    let st_b = (s_b / vectorlength) * vectorlength;

    if (i_a < st_a) && (i_b < st_b) {
        // this is the vectorized code path
        let mut v_a: __m128i;
        let mut v_b: __m128i; //, v_bmax;
                              // we load a vector from A and a vector from B
                              // v_a = _mm_lddqu_si128((__m128i *)&A[i_a]);
                              // v_b = _mm_lddqu_si128((__m128i *)&B[i_b]);
        v_a = _mm_lddqu_si128(A.add(i_a).cast::<__m128i>());
        v_b = _mm_lddqu_si128(B.add(i_b).cast::<__m128i>());
        // we have a runningmask which indicates which values from A have been
        // spotted in B, these don't get written out.
        let mut runningmask_a_found_in_b: __m128i = _mm_setzero_si128();
        /****
         * start of the main vectorized loop
         *****/
        loop {
            // afoundinb will contain a mask indicate for each entry in A
            // whether it is seen
            // in B
            let mut a_found_in_b: __m128i = _mm_cmpistrm::<CMPESTRM_CTRL>(v_b, v_a);
            let mut runningmask_a_found_in_b: __m128i =
                _mm_or_si128(runningmask_a_found_in_b, a_found_in_b);
            // we always compare the last values of A and B
            // const uint16_t a_max = A[i_a + vectorlength - 1];
            // const uint16_t b_max = B[i_b + vectorlength - 1];
            let a_max: u16 = *A.add(i_a + vectorlength - 1);
            let b_max: u16 = *B.add(i_b + vectorlength - 1);
            if a_max <= b_max {
                // Ok. In this code path, we are ready to write our v_a
                // because there is no need to read more from B, they will
                // all be large values.
                let bitmask_belongs_to_difference =
                    _mm_extract_epi32::<0>(runningmask_a_found_in_b) ^ 0xFF;
                /*** next few lines are probably expensive *****/
                // TODO aligned
                let sm16: __m128i = _mm_loadu_si128(
                    SHUFFLE_MASK
                        .as_ptr()
                        .cast::<__m128i>()
                        .offset(bitmask_belongs_to_difference as isize),
                );
                let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
                _mm_storeu_si128(C.add(count).cast::<__m128i>(), p); // can overflow
                count += bitmask_belongs_to_difference.count_ones() as usize;
                // we advance a
                i_a += vectorlength;
                if i_a == st_a {
                    // no more
                    break;
                }
                runningmask_a_found_in_b = _mm_setzero_si128();
                // v_a = _mm_lddqu_si128((__m128i *)&A[i_a]);
                v_a = _mm_lddqu_si128(A.add(i_a).cast::<__m128i>());
            }
            if b_max <= a_max {
                // in this code path, the current v_b has become useless
                i_b += vectorlength;
                if i_b == st_b {
                    break;
                }
                //v_b = _mm_lddqu_si128((__m128i *)&B[i_b]);
                v_b = _mm_lddqu_si128(B.add(i_b).cast::<__m128i>());
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
            copy_nonoverlapping(B.add(i_b), buffer.as_mut_ptr(), s_b - i_b);
            v_b = _mm_lddqu_si128(buffer.as_ptr().cast());
            let a_found_in_b: __m128i = _mm_cmpistrm::<CMPESTRM_CTRL>(v_b, v_a);
            runningmask_a_found_in_b = _mm_or_si128(runningmask_a_found_in_b, a_found_in_b);
            let bitmask_belongs_to_difference: i32 =
                _mm_extract_epi32::<0>(runningmask_a_found_in_b) ^ 0xFF;
            let sm16: __m128i = _mm_load_si128(
                SHUFFLE_MASK
                    .as_ptr()
                    .cast::<__m128i>()
                    .offset(bitmask_belongs_to_difference as isize),
            );
            let p: __m128i = _mm_shuffle_epi8(v_a, sm16);
            _mm_storeu_si128(C.add(count).cast::<__m128i>(), p); // can overflow
            count += bitmask_belongs_to_difference.count_ones() as usize;
            i_a += vectorlength;
        }
        // at this point we should have i_a == st_a and i_b == st_b
    }
    // do the tail using scalar code
    while i_a < s_a && i_b < s_b {
        let a = *A.add(i_a);
        let b = *B.add(i_b);
        if b < a {
            i_b += 1;
        } else if a < b {
            *C.add(count) = a;
            count += 1;
            i_a += 1;
        } else {
            //==
            i_a += 1;
            i_b += 1;
        }
    }
    if i_a < s_a {
        if C as *const u16 == A {
            assert!(count <= i_a);
            if count < i_a {
                copy_nonoverlapping(A.add(i_a), C.add(count), s_a - i_a);
            }
        } else {
            for i in 0..(s_a - i_a) {
                *C.add(count + i) = *A.add(i + i_a);
            }
        }
        count += s_a - i_a;
    }
    return count;
}
