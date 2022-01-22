//! ## SIMD compatibility layer
//!
//! These functions do not correspond to any LLVM intrinsic
//! so they remain unimplemented by std::simd

use std::mem;
use std::simd::{i8x16, mask16x8, u16x8, u8x16, Simd};

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

#[allow(unreachable_code)]
#[inline]
pub fn to_bitmask(mask: mask16x8) -> usize {
    #[cfg(target_feature = "sse2")]
    unsafe {
        #[cfg(target_arch = "x86")]
        use std::arch::x86::{_mm_movemask_epi8, _mm_packs_epi16, _mm_setzero_si128};
        #[cfg(target_arch = "x86_64")]
        use std::arch::x86_64::{_mm_movemask_epi8, _mm_packs_epi16, _mm_setzero_si128};
        return _mm_movemask_epi8(_mm_packs_epi16(mem::transmute(mask), _mm_setzero_si128()))
            as usize;
    }

    // TODO add neon
    // could be impl with a few neon instr
    // https://github.com/lemire/Code-used-on-Daniel-Lemire-s-blog/blob/b8257/extra/neon/movemask/code.h#L24
    // https://stackoverflow.com/questions/11870910/sse-mm-movemask-epi8-equivalent-method-for-arm-neon

    // TODO test
    #[cfg(target_feature = "simd128")]
    unsafe {
        #[cfg(target_arch = "wasm32")]
        use std::arch::wasm32::i16x8_bitmask;
        #[cfg(target_arch = "wasm64")]
        use std::arch::wasm64::i16x8_bitmask;
        return i16x8_bitmask(mem::transmute(mask)) as usize;
    }

    // fallback to scalar bitmask
    let arr = mask.to_array();
    let mut m: usize = 0;
    for i in 0..8 {
        m |= (arr[i] as usize) << i;
    }
    m
}
