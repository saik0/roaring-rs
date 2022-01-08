use std::borrow::{Borrow, BorrowMut, Cow};
use std::cmp::Ordering;
use std::collections::binary_heap::{BinaryHeap, PeekMut};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::collections::hash_map::Entry as HEntry;
use std::collections::btree_map::Entry as BEntry;
use std::mem;
use std::ops::{BitAndAssign, BitOrAssign, BitXorAssign, SubAssign};
use nohash_hasher::{BuildNoHashHasher, IntMap, NoHashHasher};
use retain_mut::RetainMut;

use crate::bitmap::container::Container;
use crate::bitmap::store::Store;
use crate::RoaringBitmap;

struct PeekedContainer<C, I> {
    container: C,
    iter: I,
}

impl<C: Borrow<Container>, I> Ord for PeekedContainer<C, I> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.container.borrow().key.cmp(&other.container.borrow().key).reverse()
    }
}

impl<C: Borrow<Container>, I> PartialOrd for PeekedContainer<C, I> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<C: Borrow<Container>, I> Eq for PeekedContainer<C, I> {}

impl<C: Borrow<Container>, I> PartialEq for PeekedContainer<C, I> {
    fn eq(&self, other: &Self) -> bool {
        self.container.borrow().key == other.container.borrow().key
    }
}

pub trait MultiBitOr<Rbs>: IntoIterator<Item=Rbs> {
    fn bitor(self) -> RoaringBitmap;
}

impl<'a, I> MultiBitOr<&'a RoaringBitmap> for I
    where
        I: IntoIterator<Item=&'a RoaringBitmap>,
{
    fn bitor(self) -> RoaringBitmap {
        let iter = self.into_iter();
        let mut heap = BinaryHeap::with_capacity(iter.size_hint().0);

        for rb in iter {
            let mut iter = rb.containers.iter();
            if let Some(container) = iter.next() {
                heap.push(PeekedContainer { container, iter });
            }
        }

        let mut containers = Vec::new();
        let mut current = None;

        while let Some(mut peek) = heap.peek_mut() {
            let pkey = peek.container.key;
            let container = match peek.iter.next() {
                Some(next) => mem::replace(&mut peek.container, next),
                None => PeekMut::pop(peek).container,
            };

            match current.as_mut() {
                Some((ckey, cstore)) => {
                    if *ckey == pkey {
                        *cstore |= &container.store;
                    } else {
                        let key = mem::replace(ckey, container.key);
                        let store = mem::replace(cstore, container.store.to_bitmap());

                        let mut container = Container { key, len: store.len(), store };
                        container.ensure_correct_store();
                        containers.push(container);
                    }
                }
                None => current = Some((container.key, container.store.to_bitmap())),
            }
        }

        if let Some((key, store)) = current {
            let mut container = Container { key, len: store.len(), store };
            container.ensure_correct_store();
            containers.push(container);
        }

        RoaringBitmap { containers }
    }
}

impl<I> MultiBitOr<RoaringBitmap> for I
    where
        I: IntoIterator<Item=RoaringBitmap>,
{
    fn bitor(self) -> RoaringBitmap {
        fn into_bitmap(store: Store) -> Store {
            match store {
                Store::Bitmap(_) => store,
                Store::Array(_) => store.to_bitmap(),
            }
        }

        let iter = self.into_iter();
        let mut heap = BinaryHeap::with_capacity(iter.size_hint().0);

        for rb in iter {
            let mut iter = rb.containers.into_iter();
            if let Some(container) = iter.next() {
                heap.push(PeekedContainer { container, iter });
            }
        }

        let mut containers = Vec::new();
        let mut current = None;

        while let Some(mut peek) = heap.peek_mut() {
            let pkey = peek.container.key;
            let container = match peek.iter.next() {
                Some(next) => mem::replace(&mut peek.container, next),
                None => PeekMut::pop(peek).container,
            };

            match current.as_mut() {
                Some((ckey, cstore)) => {
                    if *ckey == pkey {
                        *cstore |= &container.store;
                    } else {
                        let key = mem::replace(ckey, container.key);
                        let store = mem::replace(cstore, into_bitmap(container.store));

                        let mut container = Container { key, len: store.len(), store };
                        container.ensure_correct_store();
                        containers.push(container);
                    }
                }
                None => current = Some((container.key, into_bitmap(container.store))),
            }
        }

        if let Some((key, store)) = current {
            let mut container = Container { key, len: store.len(), store };
            container.ensure_correct_store();
            containers.push(container);
        }

        RoaringBitmap { containers }
    }
}

pub trait MultiBitAnd<Rbs>: IntoIterator<Item=Rbs> {
    fn bitand(self) -> RoaringBitmap;
}

impl<'a, I> MultiBitAnd<&'a RoaringBitmap> for I
    where
        I: IntoIterator<Item=&'a RoaringBitmap>,
{
    fn bitand(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next().cloned() {
            Some(mut first) => {
                for rb in iter {
                    if first.is_empty() {
                        break;
                    }
                    first &= rb;
                }
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

impl<I> MultiBitAnd<RoaringBitmap> for I
    where
        I: IntoIterator<Item=RoaringBitmap>,
{
    fn bitand(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next() {
            Some(mut first) => {
                for rb in iter {
                    if first.is_empty() {
                        break;
                    }
                    first &= rb;
                }
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

pub trait MultiBitXor<Rbs>: IntoIterator<Item=Rbs> {
    fn bitxor(self) -> RoaringBitmap;
}

impl<'a, I> MultiBitXor<&'a RoaringBitmap> for I
    where
        I: IntoIterator<Item=&'a RoaringBitmap>,
{
    fn bitxor(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next().cloned() {
            Some(mut first) => {
                iter.for_each(|rb| first ^= rb);
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

impl<I> MultiBitXor<RoaringBitmap> for I
    where
        I: IntoIterator<Item=RoaringBitmap>,
{
    fn bitxor(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next() {
            Some(mut first) => {
                iter.for_each(|rb| first ^= rb);
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

pub trait MultiSub<Rbs>: IntoIterator<Item=Rbs> {
    fn sub(self) -> RoaringBitmap;
}

impl<'a, I> MultiSub<&'a RoaringBitmap> for I
    where
        I: IntoIterator<Item=&'a RoaringBitmap>,
{
    fn sub(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next().cloned() {
            Some(mut first) => {
                iter.for_each(|rb| first -= rb);
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

impl<I> MultiSub<RoaringBitmap> for I
    where
        I: IntoIterator<Item=RoaringBitmap>,
{
    fn sub(self) -> RoaringBitmap {
        let mut iter = self.into_iter();
        match iter.next() {
            Some(mut first) => {
                iter.for_each(|rb| first -= rb);
                first
            }
            None => RoaringBitmap::default(),
        }
    }
}

pub fn naive_multi_or_cow<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_cow_btreemap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow_btreemap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_cow_hashmap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow_hashmap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_ref<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_ref_btreemap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref_btreemap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_ref_hashmap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref_hashmap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_or_owned(i: impl IntoIterator<Item=RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_owned(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_and_ref<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref(i, |a, b| BitAndAssign::bitand_assign(a, b))
}

pub fn naive_multi_and_owned(i: impl IntoIterator<Item=RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_owned(i, |a, b| BitAndAssign::bitand_assign(a, b))
}

pub fn naive_multi_sub_ref<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref(i, |a, b| SubAssign::sub_assign(a, b))
}

pub fn naive_multi_sub_owned(i: impl IntoIterator<Item=RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_owned(i, |a, b| SubAssign::sub_assign(a, b))
}

pub fn naive_multi_xor_cow<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow(i, |a, b| BitXorAssign::bitxor_assign(a, b))
}

pub fn naive_multi_xor_cow_btreemap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow_btreemap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_xor_cow_hashmap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow_hashmap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_xor_ref<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref(i, |a, b| BitXorAssign::bitxor_assign(a, b))
}

pub fn naive_multi_xor_ref_btreemap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_cow_btreemap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_xor_ref_hashmap<'a>(i: impl IntoIterator<Item=&'a RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_ref_hashmap(i, |a, b| BitOrAssign::bitor_assign(a, b))
}

pub fn naive_multi_xor_owned(i: impl IntoIterator<Item=RoaringBitmap>) -> RoaringBitmap {
    naive_lazy_multi_op_owned(i, |a, b| BitXorAssign::bitxor_assign(a, b))
}

// and so on...


#[inline]
fn naive_lazy_multi_op_owned(bitmaps: impl IntoIterator<Item=RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();
    let mut containers = match iter.next() {
        None => Vec::new(),
        Some(v) => v.containers
    };

    for bitmap in iter {
        for mut rhs in bitmap.containers {
            match containers.binary_search_by_key(&rhs.key, |c| c.key) {
                Err(loc) => {
                    containers.insert(loc, rhs)
                }
                Ok(loc) => {
                    let lhs = &mut containers[loc];
                    match (&lhs.store, &rhs.store) {
                        (Store::Array(..), Store::Array(..)) => lhs.store = lhs.store.to_bitmap(),
                        (Store::Array(..), Store::Bitmap(..)) => mem::swap(lhs, &mut rhs),
                        (Store::Bitmap(..), _) => {}
                    };
                    op(&mut lhs.store, &rhs.store);
                }
            }
        }
    }

    containers.retain_mut(|container| {
        container.len = container.store.len();
        container.ensure_correct_store();
        container.len > 0
    });

    RoaringBitmap { containers }
}

#[inline]
fn naive_lazy_multi_op_ref<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();
    let mut containers = match iter.next() {
        None => Vec::new(),
        Some(v) => v.containers.clone()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.binary_search_by_key(&rhs.key, |c| c.key) {
                Err(loc) => {
                    containers.insert(loc, rhs.clone())
                }
                Ok(loc) => {
                    let lhs = &mut containers[loc];
                    match lhs.store {
                        Store::Array(..) => { lhs.store = lhs.store.to_bitmap() }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.store, &rhs.store);
                }
            }
        }
    }

    containers.retain_mut(|container| {
        container.len = container.store.len();
        container.ensure_correct_store();
        container.len > 0
    });

    RoaringBitmap { containers }
}

// #[derive(PartialEq, Eq)]
// struct ContainerWrapper<C: Borrow<Container>>(C);
//
// impl<C: Borrow<Container> + PartialEq> PartialOrd<Self> for ContainerWrapper<C> {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.0.borrow().len.cmp(&other.0.borrow().len))
//     }
// }


#[inline]
fn naive_lazy_multi_op_ref_btreemap<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();

    let mut containers: BTreeMap<u16, Container> = match iter.next() {
        None => BTreeMap::default(),
        Some(v) => v.containers.iter().cloned().map(|c| (c.key, c)).collect()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.entry(rhs.key) {
                BEntry::Vacant(entry) => { entry.insert(rhs.clone()); }
                BEntry::Occupied(mut entry) => {
                    let lhs = entry.get_mut();
                    match lhs.store {
                        Store::Array(..) => { lhs.store = lhs.store.to_bitmap() }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.store, &rhs.store);
                }
            }
        }
    }

    let containers: Vec<Container> = containers.into_iter()
        .map(|(_, mut container)| {
            container.len = container.store.len();
            container.ensure_correct_store();
            container
        })
        .filter(|container| {
            container.len > 0
        })
        .collect();

    RoaringBitmap { containers }
}

#[inline]
fn naive_lazy_multi_op_ref_hashmap<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();

    let mut containers: IntMap<u16, Container> = match iter.next() {
        None => IntMap::default(),
        Some(v) => v.containers.iter().cloned().map(|c| (c.key, c)).collect()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.entry(rhs.key) {
                HEntry::Vacant(entry) => { entry.insert(rhs.clone()); }
                HEntry::Occupied(mut entry) => {
                    let lhs = entry.get_mut();
                    match lhs.store {
                        Store::Array(..) => { lhs.store = lhs.store.to_bitmap() }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.store, &rhs.store);
                }
            }
        }
    }

    let mut containers: Vec<Container> = containers.into_iter()
        .map(|(_, mut container)| {
            container.len = container.store.len();
            container.ensure_correct_store();
            container
        })
        .filter(|container| {
            container.len > 0
        })
        .collect();

    containers.sort_unstable_by_key(|container| container.key);

    RoaringBitmap { containers }
}

#[inline]
fn naive_lazy_multi_op_cow<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();

    let mut containers: Vec<Cow<Container>> = match iter.next() {
        None => Vec::new(),
        Some(v) => v.containers.iter().map(Cow::Borrowed).collect()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.binary_search_by_key(&rhs.key, |c| c.key) {
                Err(loc) => {
                    containers.insert(loc, Cow::Borrowed(rhs))
                }
                Ok(loc) => {
                    let lhs = &mut containers[loc];
                    match lhs.store {
                        // without cloning the lhs array, create a bitmap from it
                        Store::Array(..) => {
                            let new_container = Container {
                                key: lhs.key,
                                len: lhs.len,
                                store: lhs.store.to_bitmap(),
                            };
                            *lhs = Cow::Owned(new_container);
                        }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.to_mut().store, &rhs.store);
                }
            }
        }
    }

    let containers: Vec<Container> = containers.into_iter()
        .map(|c| {
            let mut container = c.into_owned();
            container.len = container.store.len();
            container.ensure_correct_store();
            container
        })
        .filter(|container| {
            container.len > 0
        })
        .collect();

    RoaringBitmap { containers }
}

#[inline]
fn naive_lazy_multi_op_cow_btreemap<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();

    let mut containers: BTreeMap<u16, Cow<Container>> = match iter.next() {
        None => BTreeMap::new(),
        Some(v) => v.containers.iter().map(|c| (c.key, Cow::Borrowed(c))).collect()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.entry(rhs.key) {
                BEntry::Vacant(entry) => { entry.insert(Cow::Borrowed(rhs)); }
                BEntry::Occupied(mut entry) => {
                    let lhs = entry.get_mut();
                    match lhs.store {
                        Store::Array(..) => {
                            let new_container = Container {
                                key: lhs.key,
                                len: lhs.len,
                                store: lhs.store.to_bitmap(),
                            };
                            *lhs = Cow::Owned(new_container);
                        }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.to_mut().store, &rhs.store);
                }
            }
        }
    }

    let containers: Vec<Container> = containers.into_iter()
        .map(|(_, container)| {
            let mut container = container.into_owned();
            container.len = container.store.len();
            container.ensure_correct_store();
            container
        })
        .filter(|container| {
            container.len > 0
        })
        .collect();

    RoaringBitmap { containers }
}

#[inline]
fn naive_lazy_multi_op_cow_hashmap<'a>(bitmaps: impl IntoIterator<Item=&'a RoaringBitmap>, op: fn(&mut Store, &Store)) -> RoaringBitmap {
    let mut iter = bitmaps.into_iter();

    let mut containers: IntMap<u16, Cow<Container>> = match iter.next() {
        None => IntMap::default(),
        Some(v) => v.containers.iter().map(|c| (c.key, Cow::Borrowed(c))).collect()
    };

    for bitmap in iter {
        for rhs in &bitmap.containers {
            match containers.entry(rhs.key) {
                HEntry::Vacant(entry) => { entry.insert(Cow::Borrowed(rhs)); }
                HEntry::Occupied(mut entry) => {
                    let lhs = entry.get_mut();
                    match lhs.store {
                        Store::Array(..) => {
                            let new_container = Container {
                                key: lhs.key,
                                len: lhs.len,
                                store: lhs.store.to_bitmap(),
                            };
                            *lhs = Cow::Owned(new_container);
                        }
                        Store::Bitmap(..) => {}
                    }
                    op(&mut lhs.to_mut().store, &rhs.store);
                }
            }
        }
    }

    let mut containers: Vec<Container> = containers.into_iter()
        .map(|(_, container)| {
            let mut container = container.into_owned();
            container.len = container.store.len();
            container.ensure_correct_store();
            container
        })
        .filter(|container| {
            container.len > 0
        })
        .collect();

    containers.sort_unstable_by_key(|container| container.key);

    RoaringBitmap { containers }
}
