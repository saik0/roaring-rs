extern crate roaring;

use proptest::arbitrary::any;
use proptest::collection::btree_set;
use proptest::proptest;
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

proptest! {
    #[test]
    fn proptest_and(a in btree_set(0u32..4096, ..=4096), b in btree_set(0u32..4096, ..=4096)) {
        let x = RoaringBitmap::from_sorted_iter(a.iter().cloned()).unwrap();
        let y = RoaringBitmap::from_sorted_iter(b.iter().cloned()).unwrap();

        let intersection_btree: Vec<u32> = a.intersection(&b).cloned().collect();
        let intersection_roaring: Vec<u32> = (x & y).into_iter().collect();

        assert_eq!(intersection_btree, intersection_roaring);
    }

    #[test]
    fn proptest_or(a in btree_set(0u32..4096, ..=4096), b in btree_set(0u32..4096, ..=4096)) {
        let x = RoaringBitmap::from_sorted_iter(a.iter().cloned()).unwrap();
        let y = RoaringBitmap::from_sorted_iter(b.iter().cloned()).unwrap();

        let intersection_btree: Vec<u32> = a.union(&b).cloned().collect();
        let intersection_roaring: Vec<u32> = (x | y).into_iter().collect();

        assert_eq!(intersection_btree, intersection_roaring);
    }

    #[test]
    fn proptest_sub(a in btree_set(0u32..4096, ..=4096), b in btree_set(0u32..4096, ..=4096)) {
        let x = RoaringBitmap::from_sorted_iter(a.iter().cloned()).unwrap();
        let y = RoaringBitmap::from_sorted_iter(b.iter().cloned()).unwrap();

        let intersection_btree: Vec<u32> = a.difference(&b).cloned().collect();
        let intersection_roaring: Vec<u32> = (x - y).into_iter().collect();

        assert_eq!(intersection_btree, intersection_roaring);
    }

    #[test]
    fn proptest_xor(a in btree_set(0u32..4096, ..=4096), b in btree_set(0u32..4096, ..=4096)) {
        let x = RoaringBitmap::from_sorted_iter(a.iter().cloned()).unwrap();
        let y = RoaringBitmap::from_sorted_iter(b.iter().cloned()).unwrap();

        let intersection_btree: Vec<u32> = a.symmetric_difference(&b).cloned().collect();
        let intersection_roaring: Vec<u32> = (x ^ y).into_iter().collect();

        assert_eq!(intersection_btree, intersection_roaring);
    }
}
