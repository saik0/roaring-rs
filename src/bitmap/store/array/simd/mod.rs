mod and;
mod lut;
mod or;
mod sub;
mod xor;

pub mod x86;

pub use and::and;
pub use or::or;
pub use sub::sub;
pub use xor::xor;

use core_simd::{LaneCount, Mask, Simd, SimdElement, SupportedLaneCount};

/// compute the min for each lane in `a` and `b`
#[inline]
fn lanes_min<U, const LANES: usize>(lhs: Simd<U, LANES>, rhs: Simd<U, LANES>) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    lhs.lanes_le(rhs).select(lhs, rhs)
}

/// compute the max for each lane in `a` and `b`
#[inline]
fn lanes_max<U, const LANES: usize>(lhs: Simd<U, LANES>, rhs: Simd<U, LANES>) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    lhs.lanes_gt(rhs).select(lhs, rhs)
}

/// write `v` to slice `out`
#[inline]
fn store<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
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
#[allow(dead_code)]
unsafe fn store_unchecked<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
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
#[allow(dead_code)]
unsafe fn load_unchecked<U, const LANES: usize>(src: &[U]) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    unsafe { std::ptr::read_unaligned(src as *const _ as *const Simd<U, LANES>) }
}

/// Compare all lanes in `a` to all lanes in `b`
///
/// Returns result mask will be set if any lane at `a[i]` is in any lane of `b`
///
/// ### Example
/// ```ignore
/// let a = Simd::from_array([1, 2, 3, 4, 32, 33, 34, 35]);
/// let b = Simd::from_array([2, 4, 6, 8, 10, 12, 14, 16]);
/// let result = matrix_cmp(a, b);
/// assert_eq!(result, Mask::from_array([false, true, false, true, false, false, false, false]));
/// ```
#[inline]
// It would be nice to implement this for all supported lane counts
// However, we currently only support u16x8 so it's not really necessary
fn matrix_cmp<U>(a: Simd<U, 8>, b: Simd<U, 8>) -> Mask<<U as SimdElement>::Mask, 8>
where
    U: SimdElement + PartialEq,
{
    a.lanes_eq(b)
        | a.lanes_eq(b.rotate_lanes_left::<1>())
        | a.lanes_eq(b.rotate_lanes_left::<2>())
        | a.lanes_eq(b.rotate_lanes_left::<3>())
        | a.lanes_eq(b.rotate_lanes_left::<4>())
        | a.lanes_eq(b.rotate_lanes_left::<5>())
        | a.lanes_eq(b.rotate_lanes_left::<6>())
        | a.lanes_eq(b.rotate_lanes_left::<7>())
}

use core_simd::{Swizzle2, Which, Which::First as A, Which::Second as B};

struct Shr1;
impl Swizzle2<8, 8> for Shr1 {
    const INDEX: [Which; 8] = [B(7), A(0), A(1), A(2), A(3), A(4), A(5), A(6)];
}

struct Shr2;
impl Swizzle2<8, 8> for Shr2 {
    const INDEX: [Which; 8] = [B(6), B(7), A(0), A(1), A(2), A(3), A(4), A(5)];
}

/// Assuming that a and b are sorted, returns an array of the sorted output.
/// Developed originally for merge sort using SIMD instructions.
/// Standard merge. See, e.g., Inoue and Taura, SIMD- and Cache-Friendly
/// Algorithm for Sorting an Array of Structures
fn simd_merge<U>(a: Simd<U, 8>, b: Simd<U, 8>) -> [Simd<U, 8>; 2]
where
    U: SimdElement + PartialOrd,
{
    let mut tmp: Simd<U, 8> = lanes_min(a, b);
    let mut max: Simd<U, 8> = lanes_max(a, b);
    tmp = tmp.rotate_lanes_left::<1>();
    let mut min: Simd<U, 8> = lanes_min(tmp, max);
    for _ in 0..6 {
        max = lanes_max(tmp, max);
        tmp = min.rotate_lanes_left::<1>();
        min = lanes_min(tmp, max);
    }
    max = lanes_max(tmp, max);
    min = min.rotate_lanes_left::<1>();
    [min, max]
}
