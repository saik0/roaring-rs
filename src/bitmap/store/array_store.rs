use std::borrow::{Borrow, BorrowMut};
use crate::bitmap::util::exponential_search;
use crate::bitmap::store::Store;
use crate::bitmap::store::Store::Bitmap;
use std::cmp::Ordering;
use std::cmp::Ordering::*;
use std::convert::{TryFrom, TryInto};
use std::fmt::{Display, Formatter};
use std::ops::{BitAnd, BitAndAssign, BitOr, BitXor, BitXorAssign, RangeInclusive, Sub, SubAssign};
use std::ptr::slice_from_raw_parts_mut;
use std::cell::{Cell, RefCell};
use std::pin::Pin;
use crate::bitmap::store::op_vector::{intersect_assign_vector, intersect_vector};

use super::bitmap_store::{bit, key, BitmapStore, BITMAP_LENGTH};

thread_local!(static THREAD_LOCAL_ARRAY: Cell<Option<Vec<u16>>> = Cell::new(None));


#[derive(Clone, Eq, PartialEq)]
pub struct ArrayStore {
    vec: Vec<u16>,
}

impl ArrayStore {
    pub fn new() -> ArrayStore {
        ArrayStore { vec: vec![] }
    }

    ///
    /// Create a new SortedU16Vec from a given vec
    /// It is up to the caller to ensure the vec is sorted and deduplicated
    /// Favor `try_from` / `try_into` for cases in which these invariants should be checked
    ///
    /// # Panics
    ///
    /// When debug_assertions are enabled and the above invariants are not met
    pub fn from_vec_unchecked(vec: Vec<u16>) -> ArrayStore {
        if cfg!(debug_assertions) {
            vec.try_into().unwrap()
        } else {
            ArrayStore { vec }
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit();
    }

    pub fn insert(&mut self, index: u16) -> bool {
        self.vec.binary_search(&index).map_err(|loc| self.vec.insert(loc, index)).is_err()
    }

    pub fn insert_range(&mut self, range: RangeInclusive<u16>) -> u64 {
        let start = *range.start();
        let end = *range.end();

        // Figure out the starting/ending position in the vec.
        let pos_start = self.vec.binary_search(&start).unwrap_or_else(|x| x);
        let pos_end = self
            .vec
            .binary_search_by(|p| {
                // binary search the right most position when equals
                match p.cmp(&end) {
                    Greater => Greater,
                    _ => Less,
                }
            })
            .unwrap_or_else(|x| x);

        // Overwrite the range in the middle - there's no need to take
        // into account any existing elements between start and end, as
        // they're all being added to the set.
        let dropped = self.vec.splice(pos_start..pos_end, start..=end);

        end as u64 - start as u64 + 1 - dropped.len() as u64
    }

    pub fn push(&mut self, index: u16) -> bool {
        if self.max().map_or(true, |max| max < index) {
            self.vec.push(index);
            true
        } else {
            false
        }
    }

    ///
    /// Pushes `index` at the end of the store.
    /// It is up to the caller to have validated index > self.max()
    ///
    /// # Panics
    ///
    /// If debug_assertions enabled and index is > self.max()
    pub(crate) fn push_unchecked(&mut self, index: u16) {
        if cfg!(debug_assertions) {
            if let Some(max) = self.max() {
                assert!(index > max, "store max >= index")
            }
        }
        self.vec.push(index);
    }

    pub fn remove(&mut self, index: u16) -> bool {
        self.vec.binary_search(&index).map(|loc| self.vec.remove(loc)).is_ok()
    }

    pub fn remove_range(&mut self, range: RangeInclusive<u16>) -> u64 {
        let start = *range.start();
        let end = *range.end();

        // Figure out the starting/ending position in the vec.
        let pos_start = self.vec.binary_search(&start).unwrap_or_else(|x| x);
        let pos_end = self
            .vec
            .binary_search_by(|p| {
                // binary search the right most position when equals
                match p.cmp(&end) {
                    Greater => Greater,
                    _ => Less,
                }
            })
            .unwrap_or_else(|x| x);
        self.vec.drain(pos_start..pos_end);
        (pos_end - pos_start) as u64
    }

    pub fn contains(&self, index: u16) -> bool {
        self.vec.binary_search(&index).is_ok()
    }

    pub fn is_disjoint(&self, other: &Self) -> bool {
        let (mut i1, mut i2) = (self.vec.iter(), other.vec.iter());
        let (mut value1, mut value2) = (i1.next(), i2.next());
        loop {
            match value1.and_then(|v1| value2.map(|v2| v1.cmp(v2))) {
                None => return true,
                Some(Equal) => return false,
                Some(Less) => value1 = i1.next(),
                Some(Greater) => value2 = i2.next(),
            }
        }
    }

    pub fn is_subset(&self, other: &Self) -> bool {
        let (mut i1, mut i2) = (self.iter(), other.iter());
        let (mut value1, mut value2) = (i1.next(), i2.next());
        loop {
            match (value1, value2) {
                (None, _) => return true,
                (Some(..), None) => return false,
                (Some(v1), Some(v2)) => match v1.cmp(v2) {
                    Equal => {
                        value1 = i1.next();
                        value2 = i2.next();
                    }
                    Less => return false,
                    Greater => value2 = i2.next(),
                },
            }
        }
    }

    pub fn to_bitmap_store(&self) -> BitmapStore {
        let mut bits = Box::new([0; BITMAP_LENGTH]);
        let len = self.len() as u64;

        for &index in self.iter() {
            bits[key(index)] |= 1 << bit(index);
        }
        BitmapStore::from_unchecked(len, bits)
    }

    pub fn len(&self) -> u64 {
        self.vec.len() as u64
    }

    pub fn min(&self) -> Option<u16> {
        self.vec.first().copied()
    }

    pub fn max(&self) -> Option<u16> {
        self.vec.last().copied()
    }

    pub fn iter(&self) -> std::slice::Iter<u16> {
        self.vec.iter()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<u16> {
        self.vec.into_iter()
    }

    pub fn as_slice(&self) -> &[u16] {
        &self.vec
    }
}

impl Default for ArrayStore {
    fn default() -> Self {
        ArrayStore::new()
    }
}

#[derive(Debug)]
pub struct Error {
    index: usize,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    Duplicate,
    OutOfOrder,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::Duplicate => {
                write!(f, "Duplicate element found at index: {}", self.index)
            }
            ErrorKind::OutOfOrder => {
                write!(f, "An element was out of order at index: {}", self.index)
            }
        }
    }
}

impl std::error::Error for Error {}

impl TryFrom<Vec<u16>> for ArrayStore {
    type Error = Error;

    fn try_from(value: Vec<u16>) -> Result<Self, Self::Error> {
        let mut iter = value.iter().enumerate();
        if let Some((_, mut prev)) = iter.next() {
            for (i, cur) in iter {
                match cur.cmp(prev) {
                    Less => return Err(Error { index: i, kind: ErrorKind::OutOfOrder }),
                    Equal => return Err(Error { index: i, kind: ErrorKind::Duplicate }),
                    Greater => (),
                }
                prev = cur;
            }
        }

        Ok(ArrayStore { vec: value })
    }
}

impl BitOr<Self> for &ArrayStore {
    type Output = ArrayStore;

    fn bitor(self, rhs: Self) -> Self::Output {
        or_array_array(self, rhs)
    }
}

// #[inline]
fn or_array_array(lhs: &ArrayStore, rhs: & ArrayStore) -> ArrayStore {
    let mut vec = Vec::new();

    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.vec.len() && j < rhs.vec.len() {
        let a = unsafe { lhs.vec.get_unchecked(i) };
        let b = unsafe { rhs.vec.get_unchecked(j) };
        match a.cmp(b) {
            Less => {
                vec.push(*a);
                i += 1
            },
            Greater => {
                vec.push(*b);
                j += 1
            },
            Equal => {
                vec.push(*a);
                i += 1;
                j += 1;
            }
        }
    }

    vec.extend_from_slice(&lhs.vec[i..]);
    vec.extend_from_slice(&rhs.vec[j..]);

    ArrayStore { vec }
}

impl BitAnd<Self> for &ArrayStore {
    type Output = ArrayStore;

    fn bitand(self, rhs: Self) -> Self::Output {
        and_array_array(self, rhs)
    }
}

impl BitAndAssign<&Self> for ArrayStore {
    fn bitand_assign(&mut self, rhs: &Self) {
        and_assign_array_array(self, rhs);
    }
}

impl BitAndAssign<&BitmapStore> for ArrayStore {
    fn bitand_assign(&mut self, rhs: &BitmapStore) {
        and_assign_array_bitmap(self, rhs);
    }
}

//#[inline(never)]
// #[inline]
fn and_array_array(lhs: &ArrayStore, rhs: & ArrayStore) -> ArrayStore {
    let mut vec = Vec::new();

    // Traverse both arrays
    let mut i = 0;
    let mut j = 0;
    while i < lhs.vec.len() && j < rhs.vec.len() {
        let a = unsafe { lhs.vec.get_unchecked(i) };
        let b = unsafe { rhs.vec.get_unchecked(j) };
        match a.cmp(b) {
            Less => i += 1,
            Greater => j += 1,
            Equal => {
                vec.push(*a);
                i += 1;
                j += 1;
            }
        }
    }

    ArrayStore { vec }
}

//#[inline(never)]
// #[inline]
fn and_assign_array_array(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    let mut i = 0;
    lhs.vec.retain(|x| {
        i += rhs.iter().skip(i).position(|y| y >= x).unwrap_or(rhs.vec.len());
        rhs.vec.get(i).map_or(false, |y| x == y)
    });
}

//#[inline(never)]
// #[inline]
pub fn and_assign_array_walk(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    and_assign_walk(&mut lhs.vec, rhs.as_slice());
}

//#[inline(never)]
// #[inline]
pub fn and_assign_array_run(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    and_assign_run(&mut lhs.vec, rhs.as_slice());
}

//#[inline(never)]
// #[inline]
pub fn and_assign_array_gallop(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    and_assign_gallop(&mut lhs.vec, rhs.as_slice());
}

//#[inline(never)]
// #[inline]
pub fn and_assign_array_opt(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    and_assign_opt(&mut lhs.vec, rhs.vec.as_slice())
}

//#[inline(never)]
// #[inline]
pub fn and_assign_array_opt_unsafe(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    and_assign_opt_unchecked(&mut lhs.vec, rhs.vec.as_slice())
}

pub fn and_assign_array_vector(lhs: &mut ArrayStore, rhs: & ArrayStore) {
    // unsafe { and_assign_opt_unsafe(&mut lhs.vec, rhs.vec.as_slice()) }
    const THRESHOLD: usize = 64;
    if lhs.vec.len() * THRESHOLD < rhs.vec.len() {
        intersect_skewed_small_unchecked(&mut lhs.vec, rhs.as_slice());
    } else if rhs.vec.len() * THRESHOLD < lhs.vec.len() {
        intersect_skewed_large_unchecked(rhs.as_slice(), &mut lhs.vec);
    } else {
        THREAD_LOCAL_ARRAY.with(|cell| {
            // intersect_assign_vector must reserve sufficient capacity within it's body
            // however, if a new vec does need to be allocated, ensure it's already large enough
            let mut buf = match cell.replace(None) {
                None => { Vec::with_capacity(lhs.vec.len().min(rhs.vec.len())) }
                Some(vec) => { vec }
            };
            intersect_assign_vector(lhs.as_slice(), rhs.as_slice(), &mut buf);
            std::mem::swap(&mut lhs.vec, &mut buf);
            cell.set(Some(buf));
        })
    }
}

// #[inline(never)]
fn and_assign_walk(lhs: &mut Vec<u16>, rhs: &[u16]) {
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
            i +=1;
            if i == lhs.len() { break 'outer }
        }
        while lhs[i] > rhs[j] {
            j +=1;
            if j == rhs.len() { break 'outer }
        }
        if lhs[i] == rhs[j] {
            lhs[k] = lhs[i];
            i += 1;
            j += 1;
            k += 1;
            if i == lhs.len() || j == rhs.len() { break 'outer }
        }
    }

    lhs.truncate(k);
}

/// This is called 'run' because of the two inner while loops
// If they were 'if'
#[inline]
pub fn and_assign_run_unchecked(lhs: &mut Vec<u16>, rhs: &[u16]) {
    if lhs.is_empty() || rhs.is_empty() {
        lhs.clear();
        return;
    }

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    unsafe {
        'outer: loop {
            while *lhs.get_unchecked(i) < *rhs.get_unchecked(j) {
                i +=1;
                if i == lhs.len() { break 'outer }
            }
            while *lhs.get_unchecked(i) > *rhs.get_unchecked(j) {
                j +=1;
                if j == rhs.len() { break 'outer }
            }
            if *lhs.get_unchecked(i) == *rhs.get_unchecked(j) {
                *lhs.get_unchecked_mut(k) = *lhs.get_unchecked(i);
                i += 1;
                j += 1;
                k += 1;
                if i == lhs.len() || j == rhs.len() { break 'outer }
            }
        }
    }

    lhs.truncate(k);
}

fn and_assign_gallop(lhs: &mut Vec<u16>, rhs: &[u16]) {
    // Handle degenerate cases
    if lhs.is_empty() || rhs.is_empty() {
        lhs.clear();
        return;
    }

    let mut min_gallop = MIN_GALLOP;

    let mut i = 0;
    let mut j = 0;
    let mut k = 0;

    'outer: loop {
        let mut count_lt: usize = 0; // Number of times in a row that first run won
        let mut count_gt: usize = 0; // Number of times in a row that second run won
        let mut count_eq: usize = 0; // Number of times in a row that both were equal

        loop {
            let a = unsafe { lhs.get_unchecked(i) };
            let b = unsafe { rhs.get_unchecked(j) };
            match a.cmp(b) {
                Less => {
                    i += 1;
                    count_lt += 1;
                    count_gt = 0;
                    count_eq = 0;
                    if i >= lhs.len() { break 'outer; }
                }
                Greater => {
                    j += 1;
                    count_lt = 0;
                    count_gt += 1;
                    count_eq = 0;
                    if j >= rhs.len() { break 'outer; }
                }
                Equal => {
                    if count_eq < MIN_RUN {
                        lhs[k] = *a;
                        i += 1;
                        j += 1;
                        k += 1;
                        count_lt = 0;
                        count_gt = 0;
                        count_eq += 1;
                        if i >= lhs.len() || j >= rhs.len() { break 'outer; }
                    } else {
                        let run_offset = 1 + lhs[i+1..].iter().zip(rhs[j+1..].iter())
                            .take_while(|(a, b)| a == b)
                            .count();
                        lhs[k..k+run_offset].copy_from_slice(&rhs[j..j+run_offset]);
                        i += run_offset;
                        j += run_offset;
                        k += run_offset;

                        if i >= lhs.len() || j >= rhs.len() { break 'outer; }
                        break; // break inner to reset counters
                    }
                }
            }
            if (count_lt | count_gt) >= min_gallop { break; }
        } // end walk loop

        loop {
            match exponential_search(&lhs[i..], &rhs[j]) {
                Ok(v) => {
                    lhs[k] = lhs[i+v];
                    i += v + 1;
                    j += 1;
                    k += 1;
                    count_lt = v + 1;
                    if i >= lhs.len() || j >= rhs.len() { break 'outer; }
                }
                Err(v) => {
                    i += v;
                    count_lt = v;
                    if i >= lhs.len() { break 'outer; }
                }
            };

            match exponential_search(&rhs[j..], &lhs[i]) {
                Ok(v) => {
                    lhs[k] = rhs[j+v];
                    i += 1;
                    j += v + 1;
                    k += 1;
                    count_gt = v + 1;
                    if i >= lhs.len() || j >= rhs.len() { break 'outer; }
                }
                Err(v) => {
                    j += v;
                    count_gt = v;
                    if j >= rhs.len() { break 'outer; }
                }
            };

            min_gallop = min_gallop.saturating_sub(1);
            if count_lt < MIN_GALLOP && count_gt < MIN_GALLOP { break; }
        }

        min_gallop += 2; // Penalize for leaving gallop mode
    }

    lhs.truncate(k);
}

// #[inline]
fn and_assign_array_bitmap(lhs: &mut ArrayStore, rhs: & BitmapStore) {
    lhs.vec.retain(|x| rhs.contains(*x));
}

impl Sub<Self> for &ArrayStore {
    type Output = ArrayStore;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut vec = Vec::new();

        // Traverse both arrays
        let mut i = 0;
        let mut j = 0;
        while i < self.vec.len() && j < rhs.vec.len() {
            let a = unsafe { self.vec.get_unchecked(i) };
            let b = unsafe { rhs.vec.get_unchecked(j) };
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
        vec.extend_from_slice(&self.vec[i..]);

        ArrayStore { vec }
    }
}

impl SubAssign<&Self> for ArrayStore {
    fn sub_assign(&mut self, rhs: &Self) {
        let mut i = 0;
        self.vec.retain(|x| {
            i += rhs.iter().skip(i).position(|y| y >= x).unwrap_or(rhs.vec.len());
            rhs.vec.get(i).map_or(true, |y| x != y)
        });
    }
}

impl SubAssign<&BitmapStore> for ArrayStore {
    fn sub_assign(&mut self, rhs: &BitmapStore) {
        self.vec.retain(|x| !rhs.contains(*x));
    }
}

impl BitXor<Self> for &ArrayStore {
    type Output = ArrayStore;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut vec = Vec::new();

        // Traverse both arrays
        let mut i = 0;
        let mut j = 0;
        while i < self.vec.len() && j < rhs.vec.len() {
            let a = unsafe { self.vec.get_unchecked(i) };
            let b = unsafe { rhs.vec.get_unchecked(j) };
            match a.cmp(b) {
                Less => {
                    vec.push(*a);
                    i += 1;
                }
                Greater => {
                    vec.push(*b);
                    j += 1;
                }
                Equal => {
                    i += 1;
                    j += 1;
                }
            }
        }

        // Store remaining elements of the arrays
        vec.extend_from_slice(&self.vec[i..]);
        vec.extend_from_slice(&rhs.vec[j..]);

        ArrayStore { vec }
    }
}

impl BitXorAssign<&Self> for ArrayStore {
    fn bitxor_assign(&mut self, rhs: &Self) {
        let mut i1 = 0usize;
        let mut iter2 = rhs.vec.iter();
        let mut current2 = iter2.next();
        while i1 < self.vec.len() {
            match current2.map(|c2| self.vec[i1].cmp(c2)) {
                None => break,
                Some(Less) => {
                    i1 += 1;
                }
                Some(Greater) => {
                    self.vec.insert(i1, *current2.unwrap());
                    i1 += 1;
                    current2 = iter2.next();
                }
                Some(Equal) => {
                    self.vec.remove(i1);
                    current2 = iter2.next();
                }
            }
        }
        if let Some(current) = current2 {
            self.vec.push(*current);
            self.vec.extend(iter2.cloned());
        }
    }
}

macro_rules! dev_println {
    ($($arg:tt)*) => (if cfg!(all(debug_assertions, not(test))) { println!($($arg)*); })
}

const MIN_RUN: usize = 3;
const MIN_GALLOP: usize = 7;

// #[inline]
pub fn union_gallop(mut lhs: &[u16], mut rhs: &[u16]) -> Vec<u16> {
    let mut vec = {
        let capacity = (lhs.len() + rhs.len()).min(4096);
        Vec::with_capacity(capacity)
    };

    // Handle degenerate cases
    if lhs.is_empty() || rhs.is_empty() {
        vec.extend_from_slice(&lhs);
        vec.extend_from_slice(&rhs);
        return vec;
    }

    let mut min_gallop = MIN_GALLOP;

    'outer: loop {
        let mut count1: usize = 0; // Number of times in a row that first run won
        let mut count2: usize = 0; // Number of times in a row that second run won


        // Do the straightforward thing until (if ever) one run starts
        // winning consistently.
        dev_println!("enter walk");
        loop {
            dev_println!("lhs: {:?}", lhs);
            dev_println!("rhs: {:?}", rhs);

            let a = &lhs[0];
            let b = &rhs[0];

            match a.cmp(b) {
                Less => {
                    dev_println!("l");
                    vec.push(*a);
                    lhs = &lhs[1..];
                    count1 += 1;
                    count2 = 0;
                    dev_println!("vec: {:?}\n\n", vec);
                    if lhs.is_empty() { break 'outer; }
                }
                Greater => {
                    dev_println!("r");
                    vec.push(*b);
                    rhs = &rhs[1..];
                    count1 = 0;
                    count2 += 1;
                    dev_println!("vec: {:?}\n\n", vec);
                    if rhs.is_empty() { break 'outer; }
                }
                Equal => {
                    dev_println!("l+r");
                    vec.push(*a);
                    lhs = &lhs[1..];
                    rhs = &rhs[1..];
                    count1 = 0;
                    count2 = 0;
                    dev_println!("vec: {:?}\n\n", vec);
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                }
            }
            if (count1 | count2) >= min_gallop { break; }
        }


        // One run is winning so consistently that galloping may be a
        // huge win. So try that, and continue galloping until (if ever)
        // neither run appears to be winning consistently anymore.
        dev_println!("enter gallop");
        loop {
            dev_println!("lhs: {:?}", lhs);
            dev_println!("rhs: {:?}", rhs);
            match exponential_search(&lhs, &rhs[0]) {
                Ok(v) => {
                    vec.extend_from_slice(&lhs[..v + 1]);
                    lhs = &lhs[v + 1..];
                    rhs = &rhs[1..];
                    count1 = v + 1;
                    dev_println!("galloped left {}", count1);
                    dev_println!("vec: {:?}\n\n", vec);
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                }
                Err(v) => {
                    vec.extend_from_slice(&lhs[..v]);
                    lhs = &lhs[v..];
                    count1 = v;
                    dev_println!("galloped left {}", count1);
                    dev_println!("vec: {:?}\n\n", vec);
                    if lhs.is_empty() { break 'outer; }
                }
            };

            dev_println!("lhs: {:?}", lhs);
            dev_println!("rhs: {:?}", rhs);
            match exponential_search(&rhs, &lhs[0]) {
                Ok(v) => {
                    vec.extend_from_slice(&rhs[..v + 1]);
                    rhs = &rhs[v + 1..];
                    lhs = &lhs[1..];
                    count2 = v + 1;
                    dev_println!("galloped right {}", count2);
                    dev_println!("vec: {:?}\n\n", vec);
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                }
                Err(v) => {
                    vec.extend_from_slice(&rhs[..v]);
                    rhs = &rhs[v..];
                    count2 = v;
                    dev_println!("galloped right {}", count2);
                    dev_println!("vec: {:?}\n\n", vec);
                    if rhs.is_empty() { break 'outer; }
                }
            };

            min_gallop = min_gallop.saturating_sub(1);
            if count1 < MIN_GALLOP && count2 < MIN_GALLOP { break; }
        }

        min_gallop += 2; // Penalize for leaving gallop mode
    }
    // end of 'outer loop
    dev_println!("end outer");

    // Store remaining elements of the arrays
    vec.extend_from_slice(&lhs);
    vec.extend_from_slice(&rhs);

    vec
}

// #[inline]
pub fn union_gallop_opt(mut lhs: &[u16], mut rhs: &[u16]) -> Vec<u16> {
    let mut vec = {
        let capacity = (lhs.len() + rhs.len()).min(4096);
        Vec::with_capacity(capacity)
    };

    // Handle degenerate cases
    if lhs.is_empty() || rhs.is_empty() {
        vec.extend_from_slice(&lhs);
        vec.extend_from_slice(&rhs);
        return vec;
    }

    let mut min_gallop = MIN_GALLOP;

    'outer: loop {
        let mut count_lt: usize = 0; // Number of times in a row that first run won
        let mut count_gt: usize = 0; // Number of times in a row that second run won
        let mut count_eq: usize = 0; // Number of times in a row that both were equal


        // Do the straightforward thing until (if ever) one run starts
        // winning consistently.
        loop {
            let a = unsafe { lhs.get_unchecked(0) };
            let b = unsafe { rhs.get_unchecked(0) };

            // The reason why we use if/else control flow rather than match
            // is because match reorders comparison operations, which is perf sensitive.
            let cmp = a.cmp(b);
            if cmp == Less {
                vec.push(*a);
                lhs = &lhs[1..];
                count_lt += 1;
                count_gt = 0;
                count_eq = 0;
                if lhs.is_empty() { break 'outer; }
            } else if cmp == Greater {
                vec.push(*b);
                rhs = &rhs[1..];
                count_lt = 0;
                count_gt += 1;
                count_eq = 0;
                if rhs.is_empty() { break 'outer; }
            } else {
                if count_eq < MIN_RUN {
                    vec.push(*a);
                    lhs = &lhs[1..];
                    rhs = &rhs[1..];
                    count_lt = 0;
                    count_gt = 0;
                    count_eq += 1;
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                } else {
                    let run_offset = 1 + lhs[1..].iter().zip(rhs[1..].iter())
                        .take_while(|(a, b)| a == b)
                        .count();
                    vec.extend_from_slice(&lhs[..run_offset]);
                    lhs = &lhs[run_offset..];
                    rhs = &rhs[run_offset..];
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                    break; // break inner to reset counters
                }
            }

            if (count_lt | count_gt) >= min_gallop { break; }
        }


        // One run is winning so consistently that galloping may be a
        // huge win. So try that, and continue galloping until (if ever)
        // neither run appears to be winning consistently anymore.
        loop {
            match exponential_search(&lhs, unsafe { rhs.get_unchecked(0) }) {
                Ok(i) => {
                    vec.extend_from_slice(&lhs[..i + 1]);
                    lhs = &lhs[i + 1..];
                    rhs = &rhs[1..];
                    count_lt = i + 1;
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                }
                Err(i) => {
                    vec.extend_from_slice(&lhs[..i]);
                    lhs = &lhs[i..];
                    count_lt = i;
                    if lhs.is_empty() { break 'outer; }
                }
            };

            match exponential_search(&rhs, unsafe { lhs.get_unchecked(0) }) {
                Ok(i) => {
                    vec.extend_from_slice(&rhs[..i + 1]);
                    rhs = &rhs[i + 1..];
                    lhs = &lhs[1..];
                    count_gt = i + 1;
                    if lhs.is_empty() || rhs.is_empty() { break 'outer; }
                }
                Err(i) => {
                    vec.extend_from_slice(&rhs[..i]);
                    rhs = &rhs[i..];
                    count_gt = i;
                    if rhs.is_empty() { break 'outer; }
                }
            };

            min_gallop = min_gallop.saturating_sub(1);
            if count_lt < MIN_GALLOP && count_gt < MIN_GALLOP { break; }
        }

        min_gallop += 2; // Penalize for leaving gallop mode
    }
    // end of 'outer loop



    // Store remaining elements of the arrays
    vec.extend_from_slice(&lhs);
    vec.extend_from_slice(&rhs);

    vec
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
fn binarySearch4(
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

    if n == 0 { return; }
    while n > 1 {
        let half = n / 2;
        base1 = if unsafe { *base1.get_unchecked(half) } < target1 { &base1[half..] } else { base1 };
        base2 = if unsafe { *base2.get_unchecked(half) } < target2 { &base2[half..] } else { base2 };
        base3 = if unsafe { *base3.get_unchecked(half) } < target3 { &base3[half..] } else { base3 };
        base4 = if unsafe { *base4.get_unchecked(half) } < target4 { &base4[half..] } else { base4 };
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
fn binarySearch2(
    array: &[u16],
    target1: u16,
    target2: u16,
    index1: &mut usize,
    index2: &mut usize,
) {
    let mut base1 = array;
    let mut base2 = array;
    let mut n = array.len();
    if n == 0 { return; }

    while n > 1 {
        let half = n / 2;
        base1 = if unsafe { *base1.get_unchecked(half) } < target1 { &base1[half..] } else { base1 };
        base2 = if unsafe { *base2.get_unchecked(half) } < target2 { &base2[half..] } else { base2 };
        n -= half;
    }

    *index1 = (unsafe { *base1.get_unchecked(0) } < target1) as usize + array.len() - base1.len();
    *index2 = (unsafe { *base2.get_unchecked(0) } < target2) as usize + array.len() - base2.len();
}

//#[inline(never)]
// #[inline]
fn intersect_skewed_small(small: &mut Vec<u16>, large: &[u16]) {
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
        binarySearch4(&large[idx_l..], target1, target2, target3, target4,
                      &mut index1, &mut index2, &mut index3, &mut index4);
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
        binarySearch2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
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
            _ => ()
        }
    }
    small.truncate(pos)
}

//#[inline(never)]
// #[inline]
fn intersect_skewed_small_unchecked(small: &mut Vec<u16>, large: &[u16]) {
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
            binarySearch4(&large[idx_l..], target1, target2, target3, target4,
                          &mut index1, &mut index2, &mut index3, &mut index4);
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
            binarySearch2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
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
                _ => ()
            }
        }
    }
    small.truncate(pos)
}

//#[inline(never)]
// #[inline]
fn intersect_skewed_large(small: &[u16], large: &mut Vec<u16>) {
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
        binarySearch4(&large[idx_l..], target1, target2, target3, target4,
                      &mut index1, &mut index2, &mut index3, &mut index4);
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
        binarySearch2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
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
            _ => ()
        }
    }
    large.truncate(pos)
}

// #[inline(never)]
// #[inline]
fn intersect_skewed_large_unchecked(small: &[u16], large: &mut Vec<u16>) {
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
            binarySearch4(&large[idx_l..], target1, target2, target3, target4,
                          &mut index1, &mut index2, &mut index3, &mut index4);
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
            binarySearch2(&large[idx_l..], target1, target2, &mut index1, &mut index2);
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
                _ => ()
            }
        }
    }
    large.truncate(pos)
}

//#[inline(never)]
// #[inline]
fn and_assign_opt(lhs: &mut Vec<u16>, rhs: &[u16]) {
    // const THRESHOLD: usize = 64;
    // if lhs.len() * THRESHOLD < rhs.len() {
    //     intersect_skewed_small(lhs, rhs);
    // } else if rhs.len() * THRESHOLD < lhs.len() {
    //     intersect_skewed_large(rhs, lhs);
    // } else {
    //     and_assign_run(lhs, rhs);
    // }

    unsafe {
        const THRESHOLD: usize = 64;
        if lhs.len() * THRESHOLD < rhs.len() {
            intersect_skewed_small_unchecked(lhs, rhs);
        } else if rhs.len() * THRESHOLD < lhs.len() {
            intersect_skewed_large_unchecked(rhs, lhs);
        } else {
            and_assign_run_unchecked(lhs, rhs);
        }
    }
}

//#[inline(never)]
#[inline]
fn and_assign_opt_unchecked(lhs: &mut Vec<u16>, rhs: &[u16]) {
    const THRESHOLD: usize = 64;
    if lhs.len() * THRESHOLD < rhs.len() {
        intersect_skewed_small_unchecked(lhs, rhs);
    } else if rhs.len() * THRESHOLD < lhs.len() {
        intersect_skewed_large_unchecked(rhs, lhs);
    } else {
        and_assign_run_unchecked(lhs, rhs);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitmap::store::Store;

    fn into_vec(s: Store) -> Vec<u16> {
        match s {
            Store::Array(vec) => vec.vec,
            Store::Bitmap(bits) => bits.to_array_store().vec,
        }
    }

    fn into_bitmap_store(s: Store) -> Store {
        match s {
            Store::Array(vec) => Store::Bitmap(vec.to_bitmap_store()),
            Store::Bitmap(..) => s,
        }
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn test_array_insert_invalid_range() {
        let mut store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));

        // Insert a range with start > end.
        let new = store.insert_range(6..=1);
        assert_eq!(new, 0);

        assert_eq!(into_vec(store), vec![1, 2, 8, 9]);
    }

    #[test]
    fn test_array_insert_range() {
        let mut store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));

        let new = store.insert_range(4..=5);
        assert_eq!(new, 2);

        assert_eq!(into_vec(store), vec![1, 2, 4, 5, 8, 9]);
    }

    #[test]
    fn test_array_insert_range_left_overlap() {
        let mut store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));

        let new = store.insert_range(2..=5);
        assert_eq!(new, 3);

        assert_eq!(into_vec(store), vec![1, 2, 3, 4, 5, 8, 9]);
    }

    #[test]
    fn test_array_insert_range_right_overlap() {
        let mut store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));

        let new = store.insert_range(4..=8);
        assert_eq!(new, 4);

        assert_eq!(into_vec(store), vec![1, 2, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_array_insert_range_full_overlap() {
        let mut store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));

        let new = store.insert_range(1..=9);
        assert_eq!(new, 5);

        assert_eq!(into_vec(store), vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges)]
    fn test_bitmap_insert_invalid_range() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 8, 9]));
        let mut store = into_bitmap_store(store);

        // Insert a range with start > end.
        let new = store.insert_range(6..=1);
        assert_eq!(new, 0);

        assert_eq!(into_vec(store), vec![1, 2, 8, 9]);
    }

    #[test]
    fn test_bitmap_insert_same_key_overlap() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 3, 62, 63]));
        let mut store = into_bitmap_store(store);

        let new = store.insert_range(1..=62);
        assert_eq!(new, 58);

        assert_eq!(into_vec(store), (1..64).collect::<Vec<_>>());
    }

    #[test]
    fn test_bitmap_insert_range() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 130]));
        let mut store = into_bitmap_store(store);

        let new = store.insert_range(4..=128);
        assert_eq!(new, 125);

        let mut want = vec![1, 2];
        want.extend(4..129);
        want.extend(&[130]);

        assert_eq!(into_vec(store), want);
    }

    #[test]
    fn test_bitmap_insert_range_left_overlap() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 130]));
        let mut store = into_bitmap_store(store);

        let new = store.insert_range(1..=128);
        assert_eq!(new, 126);

        let mut want = Vec::new();
        want.extend(1..129);
        want.extend(&[130]);

        assert_eq!(into_vec(store), want);
    }

    #[test]
    fn test_bitmap_insert_range_right_overlap() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 130]));
        let mut store = into_bitmap_store(store);

        let new = store.insert_range(4..=132);
        assert_eq!(new, 128);

        let mut want = vec![1, 2];
        want.extend(4..133);

        assert_eq!(into_vec(store), want);
    }

    #[test]
    fn test_bitmap_insert_range_full_overlap() {
        let store = Store::Array(ArrayStore::from_vec_unchecked(vec![1, 2, 130]));
        let mut store = into_bitmap_store(store);

        let new = store.insert_range(1..=134);
        assert_eq!(new, 131);

        let mut want = Vec::new();
        want.extend(1..135);

        assert_eq!(into_vec(store), want);
    }
}
