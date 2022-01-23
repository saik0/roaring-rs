use std::cmp::Ordering;
use std::cmp::Ordering::{Equal, Greater, Less};

pub fn or_array_walk(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut vec = Vec::new();

    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Less => {
                vec.push(*a);
                i += 1
            }
            Greater => {
                vec.push(*b);
                j += 1
            }
            Equal => {
                vec.push(*a);
                i += 1;
                j += 1;
            }
        }
    }

    vec.extend_from_slice(&lhs[i..]);
    vec.extend_from_slice(&rhs[j..]);

    vec
}

pub fn or_array_walk_mut(lhs: &[u16], rhs: &[u16], out: &mut [u16]) -> usize {
    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Less => {
                out[k] = *a;
                i += 1;
            }
            Greater => {
                out[k] = *b;
                j += 1;
            }
            Equal => {
                out[k] = *a;
                i += 1;
                j += 1;
            }
        }
        k += 1;
    }

    if i < lhs.len() {
        let n = lhs.len() - i;
        out[k..k + n].copy_from_slice(&lhs[i..]);
        k += n;
    } else if j < rhs.len() {
        let n = rhs.len() - j;
        out[k..k + n].copy_from_slice(&rhs[j..]);
        k += n;
    }

    k
}

// #[inline(never)]
pub fn and_assign_walk(lhs: &mut Vec<u16>, rhs: &[u16]) {
    let mut i = 0;
    let mut j = 0;
    let mut k = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Less => {
                i += 1;
            }
            Greater => {
                j += 1;
            }
            Equal => {
                lhs[k] = *a;
                i += 1;
                j += 1;
                k += 1;
            }
        }
    }

    lhs.truncate(k);
}

//#[inline(never)]
// #[inline]
pub fn and_assign_run(lhs: &mut Vec<u16>, rhs: &[u16]) {
    if lhs.is_empty() || rhs.is_empty() {
        lhs.clear();
        return;
    }

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    'outer: loop {
        while lhs[i] < rhs[j] {
            i += 1;
            if i == lhs.len() {
                break 'outer;
            }
        }
        while lhs[i] > rhs[j] {
            j += 1;
            if j == rhs.len() {
                break 'outer;
            }
        }
        if lhs[i] == rhs[j] {
            lhs[k] = lhs[i];
            i += 1;
            j += 1;
            k += 1;
            if i == lhs.len() || j == rhs.len() {
                break 'outer;
            }
        }
    }

    lhs.truncate(k);
}

/// This is called 'run' because of the two inner while loops
#[inline]
pub fn and_assign_run_unchecked(lhs: &mut Vec<u16>, rhs: &[u16]) {
    if lhs.is_empty() || rhs.is_empty() {
        lhs.clear();
        return;
    }

    // TODO safer to zero fill, then truncate?
    let max_len = lhs.len().max(rhs.len());
    if max_len > lhs.len() {
        lhs.reserve(max_len - lhs.len())
    }

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    unsafe {
        'outer: loop {
            while *lhs.get_unchecked(i) < *rhs.get_unchecked(j) {
                i += 1;
                if i == lhs.len() {
                    break 'outer;
                }
            }
            while *lhs.get_unchecked(i) > *rhs.get_unchecked(j) {
                j += 1;
                if j == rhs.len() {
                    break 'outer;
                }
            }
            if *lhs.get_unchecked(i) == *rhs.get_unchecked(j) {
                *lhs.get_unchecked_mut(k) = *lhs.get_unchecked(i);
                i += 1;
                j += 1;
                k += 1;
                if i == lhs.len() || j == rhs.len() {
                    break 'outer;
                }
            }
        }
        lhs.set_len(k);
    }
}

/**
 * Branchless binary search going after 4 values at once.
 * Assumes that array is sorted.
 * You have that array[*index1] >= target1, array[*index12] >= target2, ...
 * except when *index1 = n, in which case you know that all values in array are
 * smaller than target1, and so forth.
 * It has logarithmic complexity.
 */
//#[inline(never)]
// #[inline]
fn binary_search_4(
    array: &[u16],
    target1: u16,
    target2: u16,
    target3: u16,
    target4: u16,
    index1: &mut usize,
    index2: &mut usize,
    index3: &mut usize,
    index4: &mut usize,
) {
    let mut base1 = array;
    let mut base2 = array;
    let mut base3 = array;
    let mut base4 = array;
    let mut n = array.len();

    if n == 0 {
        return;
    }
    while n > 1 {
        let half = n / 2;
        base1 =
            if unsafe { *base1.get_unchecked(half) } < target1 { &base1[half..] } else { base1 };
        base2 =
            if unsafe { *base2.get_unchecked(half) } < target2 { &base2[half..] } else { base2 };
        base3 =
            if unsafe { *base3.get_unchecked(half) } < target3 { &base3[half..] } else { base3 };
        base4 =
            if unsafe { *base4.get_unchecked(half) } < target4 { &base4[half..] } else { base4 };
        n -= half;
    }
    *index1 = (unsafe { *base1.get_unchecked(0) } < target1) as usize + array.len() - base1.len();
    *index2 = (unsafe { *base2.get_unchecked(0) } < target2) as usize + array.len() - base2.len();
    *index3 = (unsafe { *base3.get_unchecked(0) } < target3) as usize + array.len() - base3.len();
    *index4 = (unsafe { *base4.get_unchecked(0) } < target4) as usize + array.len() - base4.len();
}

/**
 * Branchless binary search going after 2 values at once.
 * Assumes that array is sorted.
 * You have that array[*index1] >= target1, array[*index12] >= target2.
 * except when *index1 = n, in which case you know that all values in array are
 * smaller than target1, and so forth.
 * It has logarithmic complexity.
 */
//#[inline(never)]
// #[inline]
fn binary_search_2(
    array: &[u16],
    target1: u16,
    target2: u16,
    index1: &mut usize,
    index2: &mut usize,
) {
    let mut base1 = array;
    let mut base2 = array;
    let mut n = array.len();
    if n == 0 {
        return;
    }

    while n > 1 {
        let half = n / 2;
        base1 =
            if unsafe { *base1.get_unchecked(half) } < target1 { &base1[half..] } else { base1 };
        base2 =
            if unsafe { *base2.get_unchecked(half) } < target2 { &base2[half..] } else { base2 };
        n -= half;
    }

    *index1 = (unsafe { *base1.get_unchecked(0) } < target1) as usize + array.len() - base1.len();
    *index2 = (unsafe { *base2.get_unchecked(0) } < target2) as usize + array.len() - base2.len();
}

//#[inline(never)]
// #[inline]
pub fn intersect_skewed_small(small: &mut Vec<u16>, large: &[u16]) {
    debug_assert!(small.len() < large.len());
    let size_s = small.len();
    let size_l = large.len();

    let mut pos = 0;
    let mut idx_l = 0;
    let mut idx_s = 0;

    if 0 == size_s {
        small.clear();
        return;
    }

    let mut index1 = 0;
    let mut index2 = 0;
    let mut index3 = 0;
    let mut index4 = 0;
    while (idx_s + 4 <= size_s) && (idx_l < size_l) {
        let target1 = small[idx_s];
        let target2 = small[idx_s + 1];
        let target3 = small[idx_s + 2];
        let target4 = small[idx_s + 3];
        binary_search_4(
            &large[idx_l..],
            target1,
            target2,
            target3,
            target4,
            &mut index1,
            &mut index2,
            &mut index3,
            &mut index4,
        );
        if (index1 + idx_l < size_l) && (large[idx_l + index1] == target1) {
            small[pos] = target1;
            pos += 1;
        }
        if (index2 + idx_l < size_l) && (large[idx_l + index2] == target2) {
            small[pos] = target2;
            pos += 1;
        }
        if (index3 + idx_l < size_l) && (large[idx_l + index3] == target3) {
            small[pos] = target3;
            pos += 1;
        }
        if (index4 + idx_l < size_l) && (large[idx_l + index4] == target4) {
            small[pos] = target4;
            pos += 1;
        }
        idx_s += 4;
        idx_l += index4;
    }
    if (idx_s + 2 <= size_s) && (idx_l < size_l) {
        let target1 = small[idx_s];
        let target2 = small[idx_s + 1];
        binary_search_2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
        if (index1 + idx_l < size_l) && (large[idx_l + index1] == target1) {
            small[pos] = target1;
            pos += 1;
        }
        if (index2 + idx_l < size_l) && (large[idx_l + index2] == target2) {
            small[pos] = target2;
            pos += 1;
        }
        idx_s += 2;
        idx_l += index2;
    }
    if (idx_s < size_s) && (idx_l < size_l) {
        let val_s = small[idx_s];
        match large[idx_l..].binary_search(&val_s) {
            Ok(_) => {
                small[pos] = val_s;
                pos += 1;
            }
            _ => (),
        }
    }
    small.truncate(pos)
}

//#[inline(never)]
// #[inline]
pub fn intersect_skewed_small_unchecked(small: &mut Vec<u16>, large: &[u16]) {
    debug_assert!(small.len() < large.len());
    let size_s = small.len();
    let size_l = large.len();

    let mut pos = 0;
    let mut idx_l = 0;
    let mut idx_s = 0;

    if 0 == size_s {
        small.clear();
        return;
    }

    let mut index1 = 0;
    let mut index2 = 0;
    let mut index3 = 0;
    let mut index4 = 0;
    unsafe {
        while (idx_s + 4 <= size_s) && (idx_l < size_l) {
            let target1 = *small.get_unchecked(idx_s);
            let target2 = *small.get_unchecked(idx_s + 1);
            let target3 = *small.get_unchecked(idx_s + 2);
            let target4 = *small.get_unchecked(idx_s + 3);
            binary_search_4(
                &large[idx_l..],
                target1,
                target2,
                target3,
                target4,
                &mut index1,
                &mut index2,
                &mut index3,
                &mut index4,
            );
            if (index1 + idx_l < size_l) && (*large.get_unchecked(idx_l + index1) == target1) {
                *small.get_unchecked_mut(pos) = target1;
                pos += 1;
            }
            if (index2 + idx_l < size_l) && (*large.get_unchecked(idx_l + index2) == target2) {
                *small.get_unchecked_mut(pos) = target2;
                pos += 1;
            }
            if (index3 + idx_l < size_l) && (*large.get_unchecked(idx_l + index3) == target3) {
                *small.get_unchecked_mut(pos) = target3;
                pos += 1;
            }
            if (index4 + idx_l < size_l) && (*large.get_unchecked(idx_l + index4) == target4) {
                *small.get_unchecked_mut(pos) = target4;
                pos += 1;
            }
            idx_s += 4;
            idx_l += index4;
        }
        if (idx_s + 2 <= size_s) && (idx_l < size_l) {
            let target1 = *small.get_unchecked(idx_s);
            let target2 = *small.get_unchecked(idx_s + 1);
            binary_search_2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
            if (index1 + idx_l < size_l) && (*large.get_unchecked(idx_l + index1) == target1) {
                *small.get_unchecked_mut(pos) = target1;
                pos += 1;
            }
            if (index2 + idx_l < size_l) && (*large.get_unchecked(idx_l + index2) == target2) {
                *small.get_unchecked_mut(pos) = target2;
                pos += 1;
            }
            idx_s += 2;
            idx_l += index2;
        }
        if (idx_s < size_s) && (idx_l < size_l) {
            let val_s = small.get_unchecked(idx_s);
            match large[idx_l..].binary_search(val_s) {
                Ok(_) => {
                    *small.get_unchecked_mut(pos) = *val_s;
                    pos += 1;
                }
                _ => (),
            }
        }
    }
    small.truncate(pos)
}

//#[inline(never)]
// #[inline]
pub fn intersect_skewed_large(small: &[u16], large: &mut Vec<u16>) {
    debug_assert!(small.len() < large.len());
    let size_s = small.len();
    let size_l = large.len();

    let mut pos = 0;
    let mut idx_l = 0;
    let mut idx_s = 0;

    if 0 == size_s {
        large.clear();
        return;
    }

    let mut index1 = 0;
    let mut index2 = 0;
    let mut index3 = 0;
    let mut index4 = 0;
    while (idx_s + 4 <= size_s) && (idx_l < size_l) {
        let target1 = small[idx_s];
        let target2 = small[idx_s + 1];
        let target3 = small[idx_s + 2];
        let target4 = small[idx_s + 3];
        binary_search_4(
            &large[idx_l..],
            target1,
            target2,
            target3,
            target4,
            &mut index1,
            &mut index2,
            &mut index3,
            &mut index4,
        );
        if (index1 + idx_l < size_l) && (large[idx_l + index1] == target1) {
            large[pos] = target1;
            pos += 1;
        }
        if (index2 + idx_l < size_l) && (large[idx_l + index2] == target2) {
            large[pos] = target2;
            pos += 1;
        }
        if (index3 + idx_l < size_l) && (large[idx_l + index3] == target3) {
            large[pos] = target3;
            pos += 1;
        }
        if (index4 + idx_l < size_l) && (large[idx_l + index4] == target4) {
            large[pos] = target4;
            pos += 1;
        }
        idx_s += 4;
        idx_l += index4;
    }
    if (idx_s + 2 <= size_s) && (idx_l < size_l) {
        let target1 = small[idx_s];
        let target2 = small[idx_s + 1];
        binary_search_2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
        if (index1 + idx_l < size_l) && (large[idx_l + index1] == target1) {
            large[pos] = target1;
            pos += 1;
        }
        if (index2 + idx_l < size_l) && (large[idx_l + index2] == target2) {
            large[pos] = target2;
            pos += 1;
        }
        idx_s += 2;
        idx_l += index2;
    }
    if (idx_s < size_s) && (idx_l < size_l) {
        let val_s = small[idx_s];
        match large[idx_l..].binary_search(&val_s) {
            Ok(_) => {
                large[pos] = val_s;
                pos += 1;
            }
            _ => (),
        }
    }
    large.truncate(pos)
}

// #[inline(never)]
// #[inline]
pub fn intersect_skewed_large_unchecked(small: &[u16], large: &mut Vec<u16>) {
    debug_assert!(small.len() < large.len());
    let size_s = small.len();
    let size_l = large.len();

    let mut pos = 0;
    let mut idx_l = 0;
    let mut idx_s = 0;

    if 0 == size_s {
        large.clear();
        return;
    }

    let mut index1 = 0;
    let mut index2 = 0;
    let mut index3 = 0;
    let mut index4 = 0;
    unsafe {
        while (idx_s + 4 <= size_s) && (idx_l < size_l) {
            let target1 = *small.get_unchecked(idx_s);
            let target2 = *small.get_unchecked(idx_s + 1);
            let target3 = *small.get_unchecked(idx_s + 2);
            let target4 = *small.get_unchecked(idx_s + 3);
            binary_search_4(
                &large[idx_l..],
                target1,
                target2,
                target3,
                target4,
                &mut index1,
                &mut index2,
                &mut index3,
                &mut index4,
            );
            if (index1 + idx_l < size_l) && (*large.get_unchecked(idx_l + index1) == target1) {
                *large.get_unchecked_mut(pos) = target1;
                pos += 1;
            }
            if (index2 + idx_l < size_l) && (*large.get_unchecked(idx_l + index2) == target2) {
                *large.get_unchecked_mut(pos) = target2;
                pos += 1;
            }
            if (index3 + idx_l < size_l) && (*large.get_unchecked(idx_l + index3) == target3) {
                *large.get_unchecked_mut(pos) = target3;
                pos += 1;
            }
            if (index4 + idx_l < size_l) && (*large.get_unchecked(idx_l + index4) == target4) {
                *large.get_unchecked_mut(pos) = target4;
                pos += 1;
            }
            idx_s += 4;
            idx_l += index4;
        }
        if (idx_s + 2 <= size_s) && (idx_l < size_l) {
            let target1 = *small.get_unchecked(idx_s);
            let target2 = *small.get_unchecked(idx_s + 1);
            binary_search_2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
            if (index1 + idx_l < size_l) && (*large.get_unchecked(idx_l + index1) == target1) {
                *large.get_unchecked_mut(pos) = target1;
                pos += 1;
            }
            if (index2 + idx_l < size_l) && (*large.get_unchecked(idx_l + index2) == target2) {
                *large.get_unchecked_mut(pos) = target2;
                pos += 1;
            }
            idx_s += 2;
            idx_l += index2;
        }
        if (idx_s < size_s) && (idx_l < size_l) {
            let val_s = small.get_unchecked(idx_s);
            match large[idx_l..].binary_search(val_s) {
                Ok(_) => {
                    *large.get_unchecked_mut(pos) = *val_s;
                    pos += 1;
                }
                _ => (),
            }
        }
    }
    large.truncate(pos)
}

//#[inline(never)]
// #[inline]
pub fn and_assign_opt(lhs: &mut Vec<u16>, rhs: &[u16]) {
    const THRESHOLD: usize = 64;
    if lhs.len() * THRESHOLD < rhs.len() {
        intersect_skewed_small(lhs, rhs);
    } else if rhs.len() * THRESHOLD < lhs.len() {
        intersect_skewed_large(rhs, lhs);
    } else {
        and_assign_run(lhs, rhs);
    }
}

//#[inline(never)]
#[inline]
pub fn and_assign_opt_unchecked(lhs: &mut Vec<u16>, rhs: &[u16]) {
    const THRESHOLD: usize = 64;
    if lhs.len() * THRESHOLD < rhs.len() {
        intersect_skewed_small_unchecked(lhs, rhs);
    } else if rhs.len() * THRESHOLD < lhs.len() {
        intersect_skewed_large_unchecked(rhs, lhs);
    } else {
        and_assign_run_unchecked(lhs, rhs);
    }
}

pub fn sub_walk(lhs: &[u16], rhs: &[u16]) -> Vec<u16> {
    let mut vec = Vec::new();

    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.len() && j < rhs.len() {
        let a = unsafe { lhs.get_unchecked(i) };
        let b = unsafe { rhs.get_unchecked(j) };
        match a.cmp(b) {
            Less => {
                vec.push(*a);
                i += 1;
            }
            Greater => j += 1,
            Equal => {
                i += 1;
                j += 1;
            }
        }
    }

    // Store remaining elements of the left array
    vec.extend_from_slice(&lhs[i..]);

    vec
}

#[inline]
pub fn exponential_search<T>(slice: &[T], elem: &T) -> Result<usize, usize>
where
    T: Ord,
{
    exponential_search_by(slice, |x| x.cmp(elem))
}

#[inline]
pub fn exponential_search_by_key<T, B, F>(slice: &[T], b: &B, mut f: F) -> Result<usize, usize>
where
    F: FnMut(&T) -> B,
    B: Ord,
{
    exponential_search_by(slice, |k| f(k).cmp(b))
}

pub fn exponential_search_by<T, F>(slice: &[T], mut f: F) -> Result<usize, usize>
where
    F: FnMut(&T) -> Ordering,
{
    let mut i = 1;
    while i < slice.len() {
        // Safety: i < slice.len() by cond of while loop
        let cmp = f(unsafe { slice.get_unchecked(i) });
        if cmp == Less {
            i *= 2;
        } else if cmp == Greater {
            break;
        } else {
            return Ok(i);
        }
    }

    let lo = i / 2;
    let hi = std::cmp::min(i + 1, slice.len());

    match slice[lo..hi].binary_search_by(f) {
        Ok(j) => Ok(lo + j),
        Err(j) => Err(lo + j),
    }
}
