//! ## SIMD utilities
//!
//! functions that are generally useful, but not part of std

use std::mem;
use std::simd::{mask16x8, u16x8, LaneCount, Simd, SimdElement, SupportedLaneCount};

/// compute the min for each lane in `a` and `b`
#[inline]
pub fn lanes_min<U, const LANES: usize>(lhs: Simd<U, LANES>, rhs: Simd<U, LANES>) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    lhs.lanes_le(rhs).select(lhs, rhs)
}

/// compute the max for each lane in `a` and `b`
#[inline]
pub fn lanes_max<U, const LANES: usize>(lhs: Simd<U, LANES>, rhs: Simd<U, LANES>) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    lhs.lanes_gt(rhs).select(lhs, rhs)
}

/// write `v` to slice `out`
#[inline]
pub fn store<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    out[..LANES].copy_from_slice(&v.to_array())
}

/// write `v` to slice `out` without checking bounds
///
/// ### Safety
///   - The caller must ensure `LANES` does not exceed the allocation for `out`
#[inline]
pub unsafe fn store_unchecked<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    unsafe { std::ptr::write_unaligned(out as *mut _ as *mut Simd<U, LANES>, v) }
}

/// write `v` to slice `out` without checking bounds
///
/// ### Safety
///   - The caller must ensure `LANES` does not exceed the allocation for `out`
#[inline]
pub unsafe fn load_unchecked<U, const LANES: usize>(src: &[U]) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    unsafe { std::ptr::read_unaligned(src as *const _ as *const Simd<U, LANES>) }
}
