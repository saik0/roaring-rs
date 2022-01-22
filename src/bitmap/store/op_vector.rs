use std::cell::RefMut;
use std::mem;
use std::ptr::copy_nonoverlapping;
use std::simd::{
    i8x16, mask16x4, mask16x8, u16x16, u16x4, u16x8, u8x16, usizex8, LaneCount, Mask, Simd,
    SimdElement, SupportedLaneCount,
};

// Set operation specific helpers
