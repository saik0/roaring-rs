use std::cmp::{Ordering, Ordering::*};
use std::ops::{Bound, RangeBounds, RangeInclusive};

/// Returns the container key and the index
/// in this container for a given integer.
#[inline]
pub fn split(value: u32) -> (u16, u16) {
    ((value >> 16) as u16, value as u16)
}

/// Returns the original integer from the container
/// key and the index of it in the container.
#[inline]
pub fn join(high: u16, low: u16) -> u32 {
    (u32::from(high) << 16) + u32::from(low)
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

/// Convert a `RangeBounds<u32>` object to `RangeInclusive<u32>`,
pub fn convert_range_to_inclusive<R>(range: R) -> Option<RangeInclusive<u32>>
where
    R: RangeBounds<u32>,
{
    let start: u32 = match range.start_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&u32::MAX) => return None,
        Bound::Excluded(&i) => i + 1,
        Bound::Unbounded => 0,
    };
    let end: u32 = match range.end_bound() {
        Bound::Included(&i) => i,
        Bound::Excluded(&0) => return None,
        Bound::Excluded(&i) => i - 1,
        Bound::Unbounded => u32::MAX,
    };
    if end < start {
        return None;
    }
    Some(start..=end)
}

#[cfg(test)]
mod test {
    use super::{convert_range_to_inclusive, join, split};

    #[test]
    fn test_split_u32() {
        assert_eq!((0x0000u16, 0x0000u16), split(0x0000_0000u32));
        assert_eq!((0x0000u16, 0x0001u16), split(0x0000_0001u32));
        assert_eq!((0x0000u16, 0xFFFEu16), split(0x0000_FFFEu32));
        assert_eq!((0x0000u16, 0xFFFFu16), split(0x0000_FFFFu32));
        assert_eq!((0x0001u16, 0x0000u16), split(0x0001_0000u32));
        assert_eq!((0x0001u16, 0x0001u16), split(0x0001_0001u32));
        assert_eq!((0xFFFFu16, 0xFFFEu16), split(0xFFFF_FFFEu32));
        assert_eq!((0xFFFFu16, 0xFFFFu16), split(0xFFFF_FFFFu32));
    }

    #[test]
    fn test_join_u32() {
        assert_eq!(0x0000_0000u32, join(0x0000u16, 0x0000u16));
        assert_eq!(0x0000_0001u32, join(0x0000u16, 0x0001u16));
        assert_eq!(0x0000_FFFEu32, join(0x0000u16, 0xFFFEu16));
        assert_eq!(0x0000_FFFFu32, join(0x0000u16, 0xFFFFu16));
        assert_eq!(0x0001_0000u32, join(0x0001u16, 0x0000u16));
        assert_eq!(0x0001_0001u32, join(0x0001u16, 0x0001u16));
        assert_eq!(0xFFFF_FFFEu32, join(0xFFFFu16, 0xFFFEu16));
        assert_eq!(0xFFFF_FFFFu32, join(0xFFFFu16, 0xFFFFu16));
    }

    #[test]
    fn test_convert_range_to_inclusive() {
        assert_eq!(Some(1..=5), convert_range_to_inclusive(1..6));
        assert_eq!(Some(1..=u32::MAX), convert_range_to_inclusive(1..));
        assert_eq!(Some(0..=u32::MAX), convert_range_to_inclusive(..));
        assert_eq!(None, convert_range_to_inclusive(5..5));
        assert_eq!(Some(16..=16), convert_range_to_inclusive(16..=16))
    }
}
