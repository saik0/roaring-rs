extern crate roaring;

use roaring::RoaringBitmap;

#[test]
fn or() {
    let mut rb1 = (1..4).collect::<RoaringBitmap>();
    let rb2 = (3..6).collect::<RoaringBitmap>();
    let rb3 = (1..6).collect::<RoaringBitmap>();

    assert_eq!(rb3, &rb1 | &rb2);
    assert_eq!(rb3, &rb1 | rb2.clone());
    assert_eq!(rb3, rb1.clone() | &rb2);
    assert_eq!(rb3, rb1.clone() | rb2.clone());

    rb1 |= &rb2;
    rb1 |= rb2;

    assert_eq!(rb3, rb1);
}

#[test]
fn and() {
    let mut rb1 = (1..4).collect::<RoaringBitmap>();
    let rb2 = (3..6).collect::<RoaringBitmap>();
    let rb3 = (3..4).collect::<RoaringBitmap>();

    assert_eq!(rb3, &rb1 & &rb2);
    assert_eq!(rb3, &rb1 & rb2.clone());
    assert_eq!(rb3, rb1.clone() & &rb2);
    assert_eq!(rb3, rb1.clone() & rb2.clone());

    rb1 &= &rb2;
    rb1 &= rb2;

    assert_eq!(rb3, rb1);
}

#[test]
fn sub() {
    let mut rb1 = (1..4000).collect::<RoaringBitmap>();
    let rb2 = (3..5000).collect::<RoaringBitmap>();
    let rb3 = (1..3).collect::<RoaringBitmap>();

    assert_eq!(rb3, &rb1 - &rb2);
    assert_eq!(rb3, &rb1 - rb2.clone());
    assert_eq!(rb3, rb1.clone() - &rb2);
    assert_eq!(rb3, rb1.clone() - rb2.clone());

    rb1 -= &rb2;
    rb1 -= rb2;

    assert_eq!(rb3, rb1);
}

#[test]
fn xor() {
    let mut rb1 = (1..4).collect::<RoaringBitmap>();
    let rb2 = (3..6).collect::<RoaringBitmap>();
    let rb3 = (1..3).chain(4..6).collect::<RoaringBitmap>();
    let rb4 = (0..0).collect::<RoaringBitmap>();

    assert_eq!(rb3, &rb1 ^ &rb2);
    assert_eq!(rb3, &rb1 ^ rb2.clone());
    assert_eq!(rb3, rb1.clone() ^ &rb2);
    assert_eq!(rb3, rb1.clone() ^ rb2.clone());

    rb1 ^= &rb2;

    assert_eq!(rb3, rb1);

    rb1 ^= rb3;

    assert_eq!(rb4, rb1);
}

// Edge case for 128 bit SIMD
#[test]
fn xor_self_len_8() {
    let rb1 = (0..7).collect::<RoaringBitmap>();

    let rb2 = &rb1 ^ &rb1;

    assert_eq!(RoaringBitmap::new(), rb2);
}

// Edge case for 128 bit SIMD
#[test]
fn xor_combined_len_8192() {
    let rb1 = (0..4096).collect::<RoaringBitmap>();
    let rb2 = (4096..8192).collect::<RoaringBitmap>();

    let rb3 = &rb1 ^ &rb2;

    assert_eq!((0..8192).collect::<RoaringBitmap>(), rb3);
}

// Edge case for 128 bit SIMD
#[test]
fn or_self_len_8() {
    let rb1 = (0..8).collect::<RoaringBitmap>();

    let rb2 = &rb1 | &rb1;

    assert_eq!((0..8).collect::<RoaringBitmap>(), rb2);
}

// Edge case for 128 bit SIMD
// #[test]
// fn or_combined_len_8192() {
//     let rb1 = (0..4096).collect::<RoaringBitmap>();
//     let rb2 = (4096..8192).collect::<RoaringBitmap>();
//
//     let rb3 = &rb1 | &rb2;
//
//     assert_eq!((0..8192).collect::<RoaringBitmap>(), rb3);
// }
