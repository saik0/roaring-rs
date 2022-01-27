mod and;
mod or;
mod sub;
mod xor;

pub mod x86;

pub use and::*;
pub use or::*;
pub use sub::*;
pub use xor::*;

use core_simd::{simd_swizzle, u16x8, LaneCount, Mask, Simd, SimdElement, SupportedLaneCount};

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
pub fn store<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    debug_assert!(out.len() >= LANES);
    unsafe {
        store_unchecked(v, out);
    }
}

#[inline]
fn load<U, const LANES: usize>(src: &[U]) -> Simd<U, LANES>
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    debug_assert!(src.len() >= LANES);
    unsafe { load_unchecked(src) }
}

/// write `v` to slice `out` without checking bounds
///
/// ### Safety
///   - The caller must ensure `LANES` does not exceed the allocation for `out`
#[inline]
unsafe fn store_unchecked<U, const LANES: usize>(v: Simd<U, LANES>, out: &mut [U])
where
    U: SimdElement + PartialOrd,
    LaneCount<LANES>: SupportedLaneCount,
{
    // unsafe { std::ptr::write_unaligned(out as *mut _ as *mut [U; LANES], v.to_array()) }
    unsafe { std::ptr::write_unaligned(out as *mut _ as *mut Simd<U, LANES>, v) }
}

/// write `v` to slice `out` without checking bounds
///
/// ### Safety
///   - The caller must ensure `LANES` does not exceed the allocation for `out`
#[inline]
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
pub fn matrix_cmp<U>(a: Simd<U, 8>, b: Simd<U, 8>) -> Mask<<U as SimdElement>::Mask, 8>
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
#[inline]
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

pub fn unique_swizzle(val: u16x8, bitmask: u8) -> u16x8 {
    match bitmask {
        0x00 => simd_swizzle!(val, [0, 0, 0, 0, 0, 0, 0, 0]),
        0x01 => simd_swizzle!(val, [0, 0, 0, 0, 0, 0, 0, 0]),
        0x02 => simd_swizzle!(val, [1, 0, 0, 0, 0, 0, 0, 0]),
        0x03 => simd_swizzle!(val, [0, 1, 0, 0, 0, 0, 0, 0]),
        0x04 => simd_swizzle!(val, [2, 0, 0, 0, 0, 0, 0, 0]),
        0x05 => simd_swizzle!(val, [0, 2, 0, 0, 0, 0, 0, 0]),
        0x06 => simd_swizzle!(val, [1, 2, 0, 0, 0, 0, 0, 0]),
        0x07 => simd_swizzle!(val, [0, 1, 2, 0, 0, 0, 0, 0]),
        0x08 => simd_swizzle!(val, [3, 0, 0, 0, 0, 0, 0, 0]),
        0x09 => simd_swizzle!(val, [0, 3, 0, 0, 0, 0, 0, 0]),
        0x0A => simd_swizzle!(val, [1, 3, 0, 0, 0, 0, 0, 0]),
        0x0B => simd_swizzle!(val, [0, 1, 3, 0, 0, 0, 0, 0]),
        0x0C => simd_swizzle!(val, [2, 3, 0, 0, 0, 0, 0, 0]),
        0x0D => simd_swizzle!(val, [0, 2, 3, 0, 0, 0, 0, 0]),
        0x0E => simd_swizzle!(val, [1, 2, 3, 0, 0, 0, 0, 0]),
        0x0F => simd_swizzle!(val, [0, 1, 2, 3, 0, 0, 0, 0]),
        0x10 => simd_swizzle!(val, [4, 0, 0, 0, 0, 0, 0, 0]),
        0x11 => simd_swizzle!(val, [0, 4, 0, 0, 0, 0, 0, 0]),
        0x12 => simd_swizzle!(val, [1, 4, 0, 0, 0, 0, 0, 0]),
        0x13 => simd_swizzle!(val, [0, 1, 4, 0, 0, 0, 0, 0]),
        0x14 => simd_swizzle!(val, [2, 4, 0, 0, 0, 0, 0, 0]),
        0x15 => simd_swizzle!(val, [0, 2, 4, 0, 0, 0, 0, 0]),
        0x16 => simd_swizzle!(val, [1, 2, 4, 0, 0, 0, 0, 0]),
        0x17 => simd_swizzle!(val, [0, 1, 2, 4, 0, 0, 0, 0]),
        0x18 => simd_swizzle!(val, [3, 4, 0, 0, 0, 0, 0, 0]),
        0x19 => simd_swizzle!(val, [0, 3, 4, 0, 0, 0, 0, 0]),
        0x1A => simd_swizzle!(val, [1, 3, 4, 0, 0, 0, 0, 0]),
        0x1B => simd_swizzle!(val, [0, 1, 3, 4, 0, 0, 0, 0]),
        0x1C => simd_swizzle!(val, [2, 3, 4, 0, 0, 0, 0, 0]),
        0x1D => simd_swizzle!(val, [0, 2, 3, 4, 0, 0, 0, 0]),
        0x1E => simd_swizzle!(val, [1, 2, 3, 4, 0, 0, 0, 0]),
        0x1F => simd_swizzle!(val, [0, 1, 2, 3, 4, 0, 0, 0]),
        0x20 => simd_swizzle!(val, [5, 0, 0, 0, 0, 0, 0, 0]),
        0x21 => simd_swizzle!(val, [0, 5, 0, 0, 0, 0, 0, 0]),
        0x22 => simd_swizzle!(val, [1, 5, 0, 0, 0, 0, 0, 0]),
        0x23 => simd_swizzle!(val, [0, 1, 5, 0, 0, 0, 0, 0]),
        0x24 => simd_swizzle!(val, [2, 5, 0, 0, 0, 0, 0, 0]),
        0x25 => simd_swizzle!(val, [0, 2, 5, 0, 0, 0, 0, 0]),
        0x26 => simd_swizzle!(val, [1, 2, 5, 0, 0, 0, 0, 0]),
        0x27 => simd_swizzle!(val, [0, 1, 2, 5, 0, 0, 0, 0]),
        0x28 => simd_swizzle!(val, [3, 5, 0, 0, 0, 0, 0, 0]),
        0x29 => simd_swizzle!(val, [0, 3, 5, 0, 0, 0, 0, 0]),
        0x2A => simd_swizzle!(val, [1, 3, 5, 0, 0, 0, 0, 0]),
        0x2B => simd_swizzle!(val, [0, 1, 3, 5, 0, 0, 0, 0]),
        0x2C => simd_swizzle!(val, [2, 3, 5, 0, 0, 0, 0, 0]),
        0x2D => simd_swizzle!(val, [0, 2, 3, 5, 0, 0, 0, 0]),
        0x2E => simd_swizzle!(val, [1, 2, 3, 5, 0, 0, 0, 0]),
        0x2F => simd_swizzle!(val, [0, 1, 2, 3, 5, 0, 0, 0]),
        0x30 => simd_swizzle!(val, [4, 5, 0, 0, 0, 0, 0, 0]),
        0x31 => simd_swizzle!(val, [0, 4, 5, 0, 0, 0, 0, 0]),
        0x32 => simd_swizzle!(val, [1, 4, 5, 0, 0, 0, 0, 0]),
        0x33 => simd_swizzle!(val, [0, 1, 4, 5, 0, 0, 0, 0]),
        0x34 => simd_swizzle!(val, [2, 4, 5, 0, 0, 0, 0, 0]),
        0x35 => simd_swizzle!(val, [0, 2, 4, 5, 0, 0, 0, 0]),
        0x36 => simd_swizzle!(val, [1, 2, 4, 5, 0, 0, 0, 0]),
        0x37 => simd_swizzle!(val, [0, 1, 2, 4, 5, 0, 0, 0]),
        0x38 => simd_swizzle!(val, [3, 4, 5, 0, 0, 0, 0, 0]),
        0x39 => simd_swizzle!(val, [0, 3, 4, 5, 0, 0, 0, 0]),
        0x3A => simd_swizzle!(val, [1, 3, 4, 5, 0, 0, 0, 0]),
        0x3B => simd_swizzle!(val, [0, 1, 3, 4, 5, 0, 0, 0]),
        0x3C => simd_swizzle!(val, [2, 3, 4, 5, 0, 0, 0, 0]),
        0x3D => simd_swizzle!(val, [0, 2, 3, 4, 5, 0, 0, 0]),
        0x3E => simd_swizzle!(val, [1, 2, 3, 4, 5, 0, 0, 0]),
        0x3F => simd_swizzle!(val, [0, 1, 2, 3, 4, 5, 0, 0]),
        0x40 => simd_swizzle!(val, [6, 0, 0, 0, 0, 0, 0, 0]),
        0x41 => simd_swizzle!(val, [0, 6, 0, 0, 0, 0, 0, 0]),
        0x42 => simd_swizzle!(val, [1, 6, 0, 0, 0, 0, 0, 0]),
        0x43 => simd_swizzle!(val, [0, 1, 6, 0, 0, 0, 0, 0]),
        0x44 => simd_swizzle!(val, [2, 6, 0, 0, 0, 0, 0, 0]),
        0x45 => simd_swizzle!(val, [0, 2, 6, 0, 0, 0, 0, 0]),
        0x46 => simd_swizzle!(val, [1, 2, 6, 0, 0, 0, 0, 0]),
        0x47 => simd_swizzle!(val, [0, 1, 2, 6, 0, 0, 0, 0]),
        0x48 => simd_swizzle!(val, [3, 6, 0, 0, 0, 0, 0, 0]),
        0x49 => simd_swizzle!(val, [0, 3, 6, 0, 0, 0, 0, 0]),
        0x4A => simd_swizzle!(val, [1, 3, 6, 0, 0, 0, 0, 0]),
        0x4B => simd_swizzle!(val, [0, 1, 3, 6, 0, 0, 0, 0]),
        0x4C => simd_swizzle!(val, [2, 3, 6, 0, 0, 0, 0, 0]),
        0x4D => simd_swizzle!(val, [0, 2, 3, 6, 0, 0, 0, 0]),
        0x4E => simd_swizzle!(val, [1, 2, 3, 6, 0, 0, 0, 0]),
        0x4F => simd_swizzle!(val, [0, 1, 2, 3, 6, 0, 0, 0]),
        0x50 => simd_swizzle!(val, [4, 6, 0, 0, 0, 0, 0, 0]),
        0x51 => simd_swizzle!(val, [0, 4, 6, 0, 0, 0, 0, 0]),
        0x52 => simd_swizzle!(val, [1, 4, 6, 0, 0, 0, 0, 0]),
        0x53 => simd_swizzle!(val, [0, 1, 4, 6, 0, 0, 0, 0]),
        0x54 => simd_swizzle!(val, [2, 4, 6, 0, 0, 0, 0, 0]),
        0x55 => simd_swizzle!(val, [0, 2, 4, 6, 0, 0, 0, 0]),
        0x56 => simd_swizzle!(val, [1, 2, 4, 6, 0, 0, 0, 0]),
        0x57 => simd_swizzle!(val, [0, 1, 2, 4, 6, 0, 0, 0]),
        0x58 => simd_swizzle!(val, [3, 4, 6, 0, 0, 0, 0, 0]),
        0x59 => simd_swizzle!(val, [0, 3, 4, 6, 0, 0, 0, 0]),
        0x5A => simd_swizzle!(val, [1, 3, 4, 6, 0, 0, 0, 0]),
        0x5B => simd_swizzle!(val, [0, 1, 3, 4, 6, 0, 0, 0]),
        0x5C => simd_swizzle!(val, [2, 3, 4, 6, 0, 0, 0, 0]),
        0x5D => simd_swizzle!(val, [0, 2, 3, 4, 6, 0, 0, 0]),
        0x5E => simd_swizzle!(val, [1, 2, 3, 4, 6, 0, 0, 0]),
        0x5F => simd_swizzle!(val, [0, 1, 2, 3, 4, 6, 0, 0]),
        0x60 => simd_swizzle!(val, [5, 6, 0, 0, 0, 0, 0, 0]),
        0x61 => simd_swizzle!(val, [0, 5, 6, 0, 0, 0, 0, 0]),
        0x62 => simd_swizzle!(val, [1, 5, 6, 0, 0, 0, 0, 0]),
        0x63 => simd_swizzle!(val, [0, 1, 5, 6, 0, 0, 0, 0]),
        0x64 => simd_swizzle!(val, [2, 5, 6, 0, 0, 0, 0, 0]),
        0x65 => simd_swizzle!(val, [0, 2, 5, 6, 0, 0, 0, 0]),
        0x66 => simd_swizzle!(val, [1, 2, 5, 6, 0, 0, 0, 0]),
        0x67 => simd_swizzle!(val, [0, 1, 2, 5, 6, 0, 0, 0]),
        0x68 => simd_swizzle!(val, [3, 5, 6, 0, 0, 0, 0, 0]),
        0x69 => simd_swizzle!(val, [0, 3, 5, 6, 0, 0, 0, 0]),
        0x6A => simd_swizzle!(val, [1, 3, 5, 6, 0, 0, 0, 0]),
        0x6B => simd_swizzle!(val, [0, 1, 3, 5, 6, 0, 0, 0]),
        0x6C => simd_swizzle!(val, [2, 3, 5, 6, 0, 0, 0, 0]),
        0x6D => simd_swizzle!(val, [0, 2, 3, 5, 6, 0, 0, 0]),
        0x6E => simd_swizzle!(val, [1, 2, 3, 5, 6, 0, 0, 0]),
        0x6F => simd_swizzle!(val, [0, 1, 2, 3, 5, 6, 0, 0]),
        0x70 => simd_swizzle!(val, [4, 5, 6, 0, 0, 0, 0, 0]),
        0x71 => simd_swizzle!(val, [0, 4, 5, 6, 0, 0, 0, 0]),
        0x72 => simd_swizzle!(val, [1, 4, 5, 6, 0, 0, 0, 0]),
        0x73 => simd_swizzle!(val, [0, 1, 4, 5, 6, 0, 0, 0]),
        0x74 => simd_swizzle!(val, [2, 4, 5, 6, 0, 0, 0, 0]),
        0x75 => simd_swizzle!(val, [0, 2, 4, 5, 6, 0, 0, 0]),
        0x76 => simd_swizzle!(val, [1, 2, 4, 5, 6, 0, 0, 0]),
        0x77 => simd_swizzle!(val, [0, 1, 2, 4, 5, 6, 0, 0]),
        0x78 => simd_swizzle!(val, [3, 4, 5, 6, 0, 0, 0, 0]),
        0x79 => simd_swizzle!(val, [0, 3, 4, 5, 6, 0, 0, 0]),
        0x7A => simd_swizzle!(val, [1, 3, 4, 5, 6, 0, 0, 0]),
        0x7B => simd_swizzle!(val, [0, 1, 3, 4, 5, 6, 0, 0]),
        0x7C => simd_swizzle!(val, [2, 3, 4, 5, 6, 0, 0, 0]),
        0x7D => simd_swizzle!(val, [0, 2, 3, 4, 5, 6, 0, 0]),
        0x7E => simd_swizzle!(val, [1, 2, 3, 4, 5, 6, 0, 0]),
        0x7F => simd_swizzle!(val, [0, 1, 2, 3, 4, 5, 6, 0]),
        0x80 => simd_swizzle!(val, [7, 0, 0, 0, 0, 0, 0, 0]),
        0x81 => simd_swizzle!(val, [0, 7, 0, 0, 0, 0, 0, 0]),
        0x82 => simd_swizzle!(val, [1, 7, 0, 0, 0, 0, 0, 0]),
        0x83 => simd_swizzle!(val, [0, 1, 7, 0, 0, 0, 0, 0]),
        0x84 => simd_swizzle!(val, [2, 7, 0, 0, 0, 0, 0, 0]),
        0x85 => simd_swizzle!(val, [0, 2, 7, 0, 0, 0, 0, 0]),
        0x86 => simd_swizzle!(val, [1, 2, 7, 0, 0, 0, 0, 0]),
        0x87 => simd_swizzle!(val, [0, 1, 2, 7, 0, 0, 0, 0]),
        0x88 => simd_swizzle!(val, [3, 7, 0, 0, 0, 0, 0, 0]),
        0x89 => simd_swizzle!(val, [0, 3, 7, 0, 0, 0, 0, 0]),
        0x8A => simd_swizzle!(val, [1, 3, 7, 0, 0, 0, 0, 0]),
        0x8B => simd_swizzle!(val, [0, 1, 3, 7, 0, 0, 0, 0]),
        0x8C => simd_swizzle!(val, [2, 3, 7, 0, 0, 0, 0, 0]),
        0x8D => simd_swizzle!(val, [0, 2, 3, 7, 0, 0, 0, 0]),
        0x8E => simd_swizzle!(val, [1, 2, 3, 7, 0, 0, 0, 0]),
        0x8F => simd_swizzle!(val, [0, 1, 2, 3, 7, 0, 0, 0]),
        0x90 => simd_swizzle!(val, [4, 7, 0, 0, 0, 0, 0, 0]),
        0x91 => simd_swizzle!(val, [0, 4, 7, 0, 0, 0, 0, 0]),
        0x92 => simd_swizzle!(val, [1, 4, 7, 0, 0, 0, 0, 0]),
        0x93 => simd_swizzle!(val, [0, 1, 4, 7, 0, 0, 0, 0]),
        0x94 => simd_swizzle!(val, [2, 4, 7, 0, 0, 0, 0, 0]),
        0x95 => simd_swizzle!(val, [0, 2, 4, 7, 0, 0, 0, 0]),
        0x96 => simd_swizzle!(val, [1, 2, 4, 7, 0, 0, 0, 0]),
        0x97 => simd_swizzle!(val, [0, 1, 2, 4, 7, 0, 0, 0]),
        0x98 => simd_swizzle!(val, [3, 4, 7, 0, 0, 0, 0, 0]),
        0x99 => simd_swizzle!(val, [0, 3, 4, 7, 0, 0, 0, 0]),
        0x9A => simd_swizzle!(val, [1, 3, 4, 7, 0, 0, 0, 0]),
        0x9B => simd_swizzle!(val, [0, 1, 3, 4, 7, 0, 0, 0]),
        0x9C => simd_swizzle!(val, [2, 3, 4, 7, 0, 0, 0, 0]),
        0x9D => simd_swizzle!(val, [0, 2, 3, 4, 7, 0, 0, 0]),
        0x9E => simd_swizzle!(val, [1, 2, 3, 4, 7, 0, 0, 0]),
        0x9F => simd_swizzle!(val, [0, 1, 2, 3, 4, 7, 0, 0]),
        0xA0 => simd_swizzle!(val, [5, 7, 0, 0, 0, 0, 0, 0]),
        0xA1 => simd_swizzle!(val, [0, 5, 7, 0, 0, 0, 0, 0]),
        0xA2 => simd_swizzle!(val, [1, 5, 7, 0, 0, 0, 0, 0]),
        0xA3 => simd_swizzle!(val, [0, 1, 5, 7, 0, 0, 0, 0]),
        0xA4 => simd_swizzle!(val, [2, 5, 7, 0, 0, 0, 0, 0]),
        0xA5 => simd_swizzle!(val, [0, 2, 5, 7, 0, 0, 0, 0]),
        0xA6 => simd_swizzle!(val, [1, 2, 5, 7, 0, 0, 0, 0]),
        0xA7 => simd_swizzle!(val, [0, 1, 2, 5, 7, 0, 0, 0]),
        0xA8 => simd_swizzle!(val, [3, 5, 7, 0, 0, 0, 0, 0]),
        0xA9 => simd_swizzle!(val, [0, 3, 5, 7, 0, 0, 0, 0]),
        0xAA => simd_swizzle!(val, [1, 3, 5, 7, 0, 0, 0, 0]),
        0xAB => simd_swizzle!(val, [0, 1, 3, 5, 7, 0, 0, 0]),
        0xAC => simd_swizzle!(val, [2, 3, 5, 7, 0, 0, 0, 0]),
        0xAD => simd_swizzle!(val, [0, 2, 3, 5, 7, 0, 0, 0]),
        0xAE => simd_swizzle!(val, [1, 2, 3, 5, 7, 0, 0, 0]),
        0xAF => simd_swizzle!(val, [0, 1, 2, 3, 5, 7, 0, 0]),
        0xB0 => simd_swizzle!(val, [4, 5, 7, 0, 0, 0, 0, 0]),
        0xB1 => simd_swizzle!(val, [0, 4, 5, 7, 0, 0, 0, 0]),
        0xB2 => simd_swizzle!(val, [1, 4, 5, 7, 0, 0, 0, 0]),
        0xB3 => simd_swizzle!(val, [0, 1, 4, 5, 7, 0, 0, 0]),
        0xB4 => simd_swizzle!(val, [2, 4, 5, 7, 0, 0, 0, 0]),
        0xB5 => simd_swizzle!(val, [0, 2, 4, 5, 7, 0, 0, 0]),
        0xB6 => simd_swizzle!(val, [1, 2, 4, 5, 7, 0, 0, 0]),
        0xB7 => simd_swizzle!(val, [0, 1, 2, 4, 5, 7, 0, 0]),
        0xB8 => simd_swizzle!(val, [3, 4, 5, 7, 0, 0, 0, 0]),
        0xB9 => simd_swizzle!(val, [0, 3, 4, 5, 7, 0, 0, 0]),
        0xBA => simd_swizzle!(val, [1, 3, 4, 5, 7, 0, 0, 0]),
        0xBB => simd_swizzle!(val, [0, 1, 3, 4, 5, 7, 0, 0]),
        0xBC => simd_swizzle!(val, [2, 3, 4, 5, 7, 0, 0, 0]),
        0xBD => simd_swizzle!(val, [0, 2, 3, 4, 5, 7, 0, 0]),
        0xBE => simd_swizzle!(val, [1, 2, 3, 4, 5, 7, 0, 0]),
        0xBF => simd_swizzle!(val, [0, 1, 2, 3, 4, 5, 7, 0]),
        0xC0 => simd_swizzle!(val, [6, 7, 0, 0, 0, 0, 0, 0]),
        0xC1 => simd_swizzle!(val, [0, 6, 7, 0, 0, 0, 0, 0]),
        0xC2 => simd_swizzle!(val, [1, 6, 7, 0, 0, 0, 0, 0]),
        0xC3 => simd_swizzle!(val, [0, 1, 6, 7, 0, 0, 0, 0]),
        0xC4 => simd_swizzle!(val, [2, 6, 7, 0, 0, 0, 0, 0]),
        0xC5 => simd_swizzle!(val, [0, 2, 6, 7, 0, 0, 0, 0]),
        0xC6 => simd_swizzle!(val, [1, 2, 6, 7, 0, 0, 0, 0]),
        0xC7 => simd_swizzle!(val, [0, 1, 2, 6, 7, 0, 0, 0]),
        0xC8 => simd_swizzle!(val, [3, 6, 7, 0, 0, 0, 0, 0]),
        0xC9 => simd_swizzle!(val, [0, 3, 6, 7, 0, 0, 0, 0]),
        0xCA => simd_swizzle!(val, [1, 3, 6, 7, 0, 0, 0, 0]),
        0xCB => simd_swizzle!(val, [0, 1, 3, 6, 7, 0, 0, 0]),
        0xCC => simd_swizzle!(val, [2, 3, 6, 7, 0, 0, 0, 0]),
        0xCD => simd_swizzle!(val, [0, 2, 3, 6, 7, 0, 0, 0]),
        0xCE => simd_swizzle!(val, [1, 2, 3, 6, 7, 0, 0, 0]),
        0xCF => simd_swizzle!(val, [0, 1, 2, 3, 6, 7, 0, 0]),
        0xD0 => simd_swizzle!(val, [4, 6, 7, 0, 0, 0, 0, 0]),
        0xD1 => simd_swizzle!(val, [0, 4, 6, 7, 0, 0, 0, 0]),
        0xD2 => simd_swizzle!(val, [1, 4, 6, 7, 0, 0, 0, 0]),
        0xD3 => simd_swizzle!(val, [0, 1, 4, 6, 7, 0, 0, 0]),
        0xD4 => simd_swizzle!(val, [2, 4, 6, 7, 0, 0, 0, 0]),
        0xD5 => simd_swizzle!(val, [0, 2, 4, 6, 7, 0, 0, 0]),
        0xD6 => simd_swizzle!(val, [1, 2, 4, 6, 7, 0, 0, 0]),
        0xD7 => simd_swizzle!(val, [0, 1, 2, 4, 6, 7, 0, 0]),
        0xD8 => simd_swizzle!(val, [3, 4, 6, 7, 0, 0, 0, 0]),
        0xD9 => simd_swizzle!(val, [0, 3, 4, 6, 7, 0, 0, 0]),
        0xDA => simd_swizzle!(val, [1, 3, 4, 6, 7, 0, 0, 0]),
        0xDB => simd_swizzle!(val, [0, 1, 3, 4, 6, 7, 0, 0]),
        0xDC => simd_swizzle!(val, [2, 3, 4, 6, 7, 0, 0, 0]),
        0xDD => simd_swizzle!(val, [0, 2, 3, 4, 6, 7, 0, 0]),
        0xDE => simd_swizzle!(val, [1, 2, 3, 4, 6, 7, 0, 0]),
        0xDF => simd_swizzle!(val, [0, 1, 2, 3, 4, 6, 7, 0]),
        0xE0 => simd_swizzle!(val, [5, 6, 7, 0, 0, 0, 0, 0]),
        0xE1 => simd_swizzle!(val, [0, 5, 6, 7, 0, 0, 0, 0]),
        0xE2 => simd_swizzle!(val, [1, 5, 6, 7, 0, 0, 0, 0]),
        0xE3 => simd_swizzle!(val, [0, 1, 5, 6, 7, 0, 0, 0]),
        0xE4 => simd_swizzle!(val, [2, 5, 6, 7, 0, 0, 0, 0]),
        0xE5 => simd_swizzle!(val, [0, 2, 5, 6, 7, 0, 0, 0]),
        0xE6 => simd_swizzle!(val, [1, 2, 5, 6, 7, 0, 0, 0]),
        0xE7 => simd_swizzle!(val, [0, 1, 2, 5, 6, 7, 0, 0]),
        0xE8 => simd_swizzle!(val, [3, 5, 6, 7, 0, 0, 0, 0]),
        0xE9 => simd_swizzle!(val, [0, 3, 5, 6, 7, 0, 0, 0]),
        0xEA => simd_swizzle!(val, [1, 3, 5, 6, 7, 0, 0, 0]),
        0xEB => simd_swizzle!(val, [0, 1, 3, 5, 6, 7, 0, 0]),
        0xEC => simd_swizzle!(val, [2, 3, 5, 6, 7, 0, 0, 0]),
        0xED => simd_swizzle!(val, [0, 2, 3, 5, 6, 7, 0, 0]),
        0xEE => simd_swizzle!(val, [1, 2, 3, 5, 6, 7, 0, 0]),
        0xEF => simd_swizzle!(val, [0, 1, 2, 3, 5, 6, 7, 0]),
        0xF0 => simd_swizzle!(val, [4, 5, 6, 7, 0, 0, 0, 0]),
        0xF1 => simd_swizzle!(val, [0, 4, 5, 6, 7, 0, 0, 0]),
        0xF2 => simd_swizzle!(val, [1, 4, 5, 6, 7, 0, 0, 0]),
        0xF3 => simd_swizzle!(val, [0, 1, 4, 5, 6, 7, 0, 0]),
        0xF4 => simd_swizzle!(val, [2, 4, 5, 6, 7, 0, 0, 0]),
        0xF5 => simd_swizzle!(val, [0, 2, 4, 5, 6, 7, 0, 0]),
        0xF6 => simd_swizzle!(val, [1, 2, 4, 5, 6, 7, 0, 0]),
        0xF7 => simd_swizzle!(val, [0, 1, 2, 4, 5, 6, 7, 0]),
        0xF8 => simd_swizzle!(val, [3, 4, 5, 6, 7, 0, 0, 0]),
        0xF9 => simd_swizzle!(val, [0, 3, 4, 5, 6, 7, 0, 0]),
        0xFA => simd_swizzle!(val, [1, 3, 4, 5, 6, 7, 0, 0]),
        0xFB => simd_swizzle!(val, [0, 1, 3, 4, 5, 6, 7, 0]),
        0xFC => simd_swizzle!(val, [2, 3, 4, 5, 6, 7, 0, 0]),
        0xFD => simd_swizzle!(val, [0, 2, 3, 4, 5, 6, 7, 0]),
        0xFE => simd_swizzle!(val, [1, 2, 3, 4, 5, 6, 7, 0]),
        0xFF => simd_swizzle!(val, [0, 1, 2, 3, 4, 5, 6, 7]),
    }
}
