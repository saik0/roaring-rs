//! ## SIMD compatibility layer
//!
//! These functions do not correspond to any LLVM intrinsic
//! so they remain unimplemented by core_simd

use core_simd::{mask16x8, u16x8, u8x16, Simd};
use std::mem;

#[allow(unreachable_code)]
#[inline]
pub fn swizzle_bytes(a: u8x16, b: u8x16) -> u8x16 {
    #[cfg(target_feature = "ssse3")]
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::_mm_shuffle_epi8;
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::_mm_shuffle_epi8;
        return _mm_shuffle_epi8(a.into(), b.into()).into();
    }

    // TODO neon
    // https://doc.rust-lang.org/std/arch/arm/fn.vtbl2_u8.html

    // TODO test
    #[cfg(target_feature = "simd128")]
    unsafe {
        #[cfg(target_arch = "wasm32")]
        use std::arch::wasm32::u8x16_swizzle;
        #[cfg(target_arch = "wasm64")]
        use std::arch::wasm64::u8x16_swizzle;
        return mem::transmute(u8x16_swizzle(mem::transmute(a), mem::transmute(b)));
    }

    // fallback scalar shuffle
    let (a, b) = unsafe {
        (mem::transmute::<u8x16, u8x16>(a).to_array(), mem::transmute::<u8x16, u8x16>(b).to_array())
    };
    let mut r = [0; 16];
    for i in 0..16 {
        if b[i] & 0x80 == 0u8 {
            r[i] = a[(b[i] % 16) as usize];
        }
    }
    let res: u8x16 = Simd::from_array(r);
    unsafe { mem::transmute(res) }
}

#[inline]
pub fn swizzle_u16x8(a: u16x8, b: u16x8) -> u16x8 {
    unsafe { mem::transmute(swizzle_bytes(mem::transmute(a), mem::transmute(b))) }
}

#[inline]
pub fn to_bitmask(mask: mask16x8) -> usize {
    mask.to_bitmask()[0] as usize
}
