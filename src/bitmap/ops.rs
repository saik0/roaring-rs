use std::mem;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Sub, SubAssign};

use retain_mut::RetainMut;

use crate::bitmap::container::Container;

use crate::bitmap::Pairs;
use crate::RoaringBitmap;

impl RoaringBitmap {
    /// Unions in-place with the specified other bitmap.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (1..5).collect();
    ///
    /// rb1 |= rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    ///
    /// Can also be done via the `BitOr` operator.
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (1..5).collect();
    ///
    /// let rb1 = rb1 | rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    #[deprecated(
        since = "0.6.7",
        note = "Please use the `BitOrAssign::bitor_assign` (`|=`) ops method instead"
    )]
    pub fn union_with(&mut self, other: &RoaringBitmap) {
        BitOrAssign::bitor_assign(self, other)
    }

    /// Intersects in-place with the specified other bitmap.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (3..4).collect();
    ///
    /// rb1 &= rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    ///
    /// Can also be done via the `BitAnd` operator.
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (3..4).collect();
    ///
    /// let rb1 = rb1 & rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    #[deprecated(
        since = "0.6.7",
        note = "Please use the `BitAndAssign::bitand_assign` (`&=`) ops method instead"
    )]
    pub fn intersect_with(&mut self, other: &RoaringBitmap) {
        BitAndAssign::bitand_assign(self, other)
    }

    /// Removes all values in the specified other bitmap from self, in-place.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (1..3).collect();
    ///
    /// rb1 -= rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    ///
    /// Can also be done via the `Sub` operator.
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..5).collect();
    /// let rb3: RoaringBitmap = (1..3).collect();
    ///
    /// let rb1 = rb1 - rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    #[deprecated(
        since = "0.6.7",
        note = "Please use the `SubAssign::sub_assign` (`-=`) ops method instead"
    )]
    pub fn difference_with(&mut self, other: &RoaringBitmap) {
        SubAssign::sub_assign(self, other)
    }

    /// Replaces this bitmap with one that is equivalent to `self XOR other`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..6).collect();
    /// let rb3: RoaringBitmap = (1..3).chain(4..6).collect();
    ///
    /// rb1 ^= rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    ///
    /// Can also be done via the `BitXor` operator.
    ///
    /// ```rust
    /// use roaring::RoaringBitmap;
    ///
    /// let mut rb1: RoaringBitmap = (1..4).collect();
    /// let rb2: RoaringBitmap = (3..6).collect();
    /// let rb3: RoaringBitmap = (1..3).chain(4..6).collect();
    ///
    /// let rb1 = rb1 ^ rb2;
    ///
    /// assert_eq!(rb1, rb3);
    /// ```
    #[deprecated(
        since = "0.6.7",
        note = "Please use the `BitXorAssign::bitxor_assign` (`^=`) ops method instead"
    )]
    pub fn symmetric_difference_with(&mut self, other: &RoaringBitmap) {
        BitXorAssign::bitxor_assign(self, other)
    }

    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
    // ░░░░░░░░░█████╗░██████╗░░░░░░░░░
    // ░░░░░░░░██╔══██╗██╔══██╗░░░░░░░░
    // ░░░░░░░░██║░░██║██████╔╝░░░░░░░░
    // ░░░░░░░░██║░░██║██╔══██╗░░░░░░░░
    // ░░░░░░░░╚█████╔╝██║░░██║░░░░░░░░
    // ░░░░░░░░░╚════╝░╚═╝░░╚═╝░░░░░░░░
    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn or_cur(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(rhs)) => containers.push(rhs.clone()),
                (Some(lhs), Some(rhs)) => containers.push(BitOr::bitor(lhs, rhs)),
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    fn or_assign_cur(&mut self, rhs: &RoaringBitmap) {
        for container in &rhs.containers {
            let key = container.key;
            match self.containers.binary_search_by_key(&key, |c| c.key) {
                Err(loc) => self.containers.insert(loc, container.clone()),
                Ok(loc) => BitOrAssign::bitor_assign(&mut self.containers[loc], container),
            }
        }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn or_x86_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(rhs)) => containers.push(rhs.clone()),
                (Some(lhs), Some(rhs)) => containers.push(lhs.or_x86_simd(rhs)),
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn or_assign_x86_simd(&mut self, rhs: &RoaringBitmap) {
        for container in &rhs.containers {
            let key = container.key;
            match self.containers.binary_search_by_key(&key, |c| c.key) {
                Err(loc) => self.containers.insert(loc, container.clone()),
                Ok(loc) => self.containers[loc].or_assign_x86_simd(container),
            }
        }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn or_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(rhs)) => containers.push(rhs.clone()),
                (Some(lhs), Some(rhs)) => containers.push(lhs.or_simd(rhs)),
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn or_assign_simd(&mut self, rhs: &RoaringBitmap) {
        for container in &rhs.containers {
            let key = container.key;
            match self.containers.binary_search_by_key(&key, |c| c.key) {
                Err(loc) => self.containers.insert(loc, container.clone()),
                Ok(loc) => self.containers[loc].or_assign_simd(container),
            }
        }
    }

    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
    // ░░░░░░░░░█████╗░███╗░░██╗██████╗░░░░░░░░░
    // ░░░░░░░░██╔══██╗████╗░██║██╔══██╗░░░░░░░░
    // ░░░░░░░░███████║██╔██╗██║██║░░██║░░░░░░░░
    // ░░░░░░░░██╔══██║██║╚████║██║░░██║░░░░░░░░
    // ░░░░░░░░██║░░██║██║░╚███║██████╔╝░░░░░░░░
    // ░░░░░░░░╚═╝░░╚═╝╚═╝░░╚══╝╚═════╝░░░░░░░░░
    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_cur(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            if let (Some(lhs), Some(rhs)) = pair {
                let container = BitAnd::bitand(lhs, rhs);
                if container.len() != 0 {
                    containers.push(container);
                }
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_cur(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    BitAndAssign::bitand_assign(cont, &rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_walk(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    cont.and_assign_walk(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_run(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    cont.and_assign_run(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_opt_unsafe(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            if let (Some(lhs), Some(rhs)) = pair {
                let container = lhs.and_opt_unsafe(rhs);
                if container.len() != 0 {
                    containers.push(container);
                }
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_opt_unsafe(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    cont.and_assign_opt_unsafe(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_x86_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            if let (Some(lhs), Some(rhs)) = pair {
                let container = lhs.and_x86_simd(rhs);
                if container.len() != 0 {
                    containers.push(container);
                }
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_x86_simd(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    cont.and_assign_x86_simd(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            if let (Some(lhs), Some(rhs)) = pair {
                let container = lhs.and_simd(rhs);
                if container.len() != 0 {
                    containers.push(container);
                }
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn and_assign_simd(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            let key = cont.key;
            match rhs.containers.binary_search_by_key(&key, |c| c.key) {
                Ok(loc) => {
                    cont.and_assign_simd(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => false,
            }
        })
    }

    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
    // ░░░░░░░░░██████╗██╗░░░██╗██████╗░░░░░░░░░
    // ░░░░░░░░██╔════╝██║░░░██║██╔══██╗░░░░░░░░
    // ░░░░░░░░╚█████╗░██║░░░██║██████╦╝░░░░░░░░
    // ░░░░░░░░░╚═══██╗██║░░░██║██╔══██╗░░░░░░░░
    // ░░░░░░░░██████╔╝╚██████╔╝██████╦╝░░░░░░░░
    // ░░░░░░░░╚═════╝░░╚═════╝░╚═════╝░░░░░░░░░
    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_cur(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(_)) => (),
                (Some(lhs), Some(rhs)) => {
                    let container = Sub::sub(lhs, rhs);
                    if container.len() != 0 {
                        containers.push(container);
                    }
                }
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_assign_cur(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            match rhs.containers.binary_search_by_key(&cont.key, |c| c.key) {
                Ok(loc) => {
                    SubAssign::sub_assign(cont, &rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => true,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_x86_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(_)) => (),
                (Some(lhs), Some(rhs)) => {
                    let container = lhs.sub_x86_simd(rhs);
                    if container.len() != 0 {
                        containers.push(container);
                    }
                }
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_assign_x86_simd(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            match rhs.containers.binary_search_by_key(&cont.key, |c| c.key) {
                Ok(loc) => {
                    cont.sub_assign_x86_simd(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => true,
            }
        })
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(_)) => (),
                (Some(lhs), Some(rhs)) => {
                    let container = lhs.sub_simd(rhs);
                    if container.len() != 0 {
                        containers.push(container);
                    }
                }
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn sub_assign_simd(&mut self, rhs: &RoaringBitmap) {
        RetainMut::retain_mut(&mut self.containers, |cont| {
            match rhs.containers.binary_search_by_key(&cont.key, |c| c.key) {
                Ok(loc) => {
                    cont.sub_assign_simd(&rhs.containers[loc]);
                    cont.len() != 0
                }
                Err(_) => true,
            }
        })
    }

    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
    // ░░░░░░░░██╗░░██╗░█████╗░██████╗░░░░░░░░░
    // ░░░░░░░░╚██╗██╔╝██╔══██╗██╔══██╗░░░░░░░░
    // ░░░░░░░░░╚███╔╝░██║░░██║██████╔╝░░░░░░░░
    // ░░░░░░░░░██╔██╗░██║░░██║██╔══██╗░░░░░░░░
    // ░░░░░░░░██╔╝╚██╗╚█████╔╝██║░░██║░░░░░░░░
    // ░░░░░░░░╚═╝░░╚═╝░╚════╝░╚═╝░░╚═╝░░░░░░░░
    // ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn xor_cur(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(rhs)) => containers.push(rhs.clone()),
                (Some(lhs), Some(rhs)) => {
                    let container = BitXor::bitxor(lhs, rhs);
                    if container.len() != 0 {
                        containers.push(container);
                    }
                }
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn xor_assign_cur(&mut self, rhs: &RoaringBitmap) {
        for pair in Pairs::new(mem::take(&mut self.containers), &rhs.containers) {
            match pair {
                (Some(mut lhs), Some(rhs)) => {
                    BitXorAssign::bitxor_assign(&mut lhs, rhs);
                    if lhs.len() != 0 {
                        self.containers.push(lhs);
                    }
                }
                (Some(lhs), None) => self.containers.push(lhs),
                (None, Some(rhs)) => self.containers.push(rhs.clone()),
                (None, None) => break,
            }
        }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn xor_simd(&self, rhs: &RoaringBitmap) -> RoaringBitmap {
        let mut containers = Vec::new();

        for pair in Pairs::new(&self.containers, &rhs.containers) {
            match pair {
                (Some(lhs), None) => containers.push(lhs.clone()),
                (None, Some(rhs)) => containers.push(rhs.clone()),
                (Some(lhs), Some(rhs)) => {
                    let container = lhs.xor_std_simd(rhs);
                    if container.len() != 0 {
                        containers.push(container);
                    }
                }
                (None, None) => break,
            }
        }

        RoaringBitmap { containers }
    }

    #[allow(missing_docs)] // TODO Remove. This is for benchmark cmp.
    pub fn xor_assign_simd(&mut self, rhs: &RoaringBitmap) {
        for pair in Pairs::new(mem::take(&mut self.containers), &rhs.containers) {
            match pair {
                (Some(mut lhs), Some(rhs)) => {
                    lhs.xor_assign_simd(rhs);
                    if lhs.len() != 0 {
                        self.containers.push(lhs);
                    }
                }
                (Some(lhs), None) => self.containers.push(lhs),
                (None, Some(rhs)) => self.containers.push(rhs.clone()),
                (None, None) => break,
            }
        }
    }
}

impl BitOr<RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `union` between two sets.
    fn bitor(mut self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitOrAssign::bitor_assign(&mut self, rhs);
        self
    }
}

impl BitOr<&RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `union` between two sets.
    fn bitor(mut self, rhs: &RoaringBitmap) -> RoaringBitmap {
        BitOrAssign::bitor_assign(&mut self, rhs);
        self
    }
}

impl BitOr<RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `union` between two sets.
    fn bitor(self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitOr::bitor(rhs, self)
    }
}

impl BitOr<&RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `union` between two sets.
    fn bitor(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.or_cur(rhs)
    }
}

impl BitOrAssign<RoaringBitmap> for RoaringBitmap {
    /// An `union` between two sets.
    fn bitor_assign(&mut self, mut rhs: RoaringBitmap) {
        // TODO this removed a optimization where we steal containers.
        // DO NOT MERGE
        self.or_assign_cur(&rhs)
    }
}

impl BitOrAssign<&RoaringBitmap> for RoaringBitmap {
    /// An `union` between two sets.
    fn bitor_assign(&mut self, rhs: &RoaringBitmap) {
        self.or_assign_cur(rhs)
    }
}

impl BitAnd<RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `intersection` between two sets.
    fn bitand(mut self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitAndAssign::bitand_assign(&mut self, rhs);
        self
    }
}

impl BitAnd<&RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `intersection` between two sets.
    fn bitand(mut self, rhs: &RoaringBitmap) -> RoaringBitmap {
        BitAndAssign::bitand_assign(&mut self, rhs);
        self
    }
}

impl BitAnd<RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `intersection` between two sets.
    fn bitand(self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitAnd::bitand(rhs, self)
    }
}

impl BitAnd<&RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// An `intersection` between two sets.
    fn bitand(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.and_cur(rhs)
    }
}

impl BitAndAssign<RoaringBitmap> for RoaringBitmap {
    /// An `intersection` between two sets.
    fn bitand_assign(&mut self, mut rhs: RoaringBitmap) {
        // We make sure that we apply the intersection operation on the smallest map.
        if rhs.len() < self.len() {
            mem::swap(self, &mut rhs);
        }

        // TODO this removed a optimization where we steal containers.

        self.and_assign_cur(&rhs)
    }
}

impl BitAndAssign<&RoaringBitmap> for RoaringBitmap {
    /// An `intersection` between two sets.
    fn bitand_assign(&mut self, rhs: &RoaringBitmap) {
        self.and_assign_cur(rhs)
    }
}

impl Sub<RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `difference` between two sets.
    fn sub(mut self, rhs: RoaringBitmap) -> RoaringBitmap {
        SubAssign::sub_assign(&mut self, &rhs);
        self
    }
}

impl Sub<&RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `difference` between two sets.
    fn sub(mut self, rhs: &RoaringBitmap) -> RoaringBitmap {
        SubAssign::sub_assign(&mut self, rhs);
        self
    }
}

impl Sub<RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `difference` between two sets.
    fn sub(self, rhs: RoaringBitmap) -> RoaringBitmap {
        Sub::sub(self, &rhs)
    }
}

impl Sub<&RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `difference` between two sets.
    fn sub(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.sub_cur(rhs)
    }
}

impl SubAssign<RoaringBitmap> for RoaringBitmap {
    /// A `difference` between two sets.
    fn sub_assign(&mut self, rhs: RoaringBitmap) {
        SubAssign::sub_assign(self, &rhs)
    }
}

impl SubAssign<&RoaringBitmap> for RoaringBitmap {
    /// A `difference` between two sets.
    fn sub_assign(&mut self, rhs: &RoaringBitmap) {
        self.sub_assign_cur(rhs)
    }
}

impl BitXor<RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `symmetric difference` between two sets.
    fn bitxor(mut self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitXorAssign::bitxor_assign(&mut self, rhs);
        self
    }
}

impl BitXor<&RoaringBitmap> for RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `symmetric difference` between two sets.
    fn bitxor(mut self, rhs: &RoaringBitmap) -> RoaringBitmap {
        BitXorAssign::bitxor_assign(&mut self, rhs);
        self
    }
}

impl BitXor<RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `symmetric difference` between two sets.
    fn bitxor(self, rhs: RoaringBitmap) -> RoaringBitmap {
        BitXor::bitxor(rhs, self)
    }
}

impl BitXor<&RoaringBitmap> for &RoaringBitmap {
    type Output = RoaringBitmap;

    /// A `symmetric difference` between two sets.
    fn bitxor(self, rhs: &RoaringBitmap) -> RoaringBitmap {
        self.xor_cur(rhs)
    }
}

impl BitXorAssign<RoaringBitmap> for RoaringBitmap {
    /// A `symmetric difference` between two sets.
    fn bitxor_assign(&mut self, rhs: RoaringBitmap) {
        self.xor_assign_cur(&rhs)
    }
}

impl BitXorAssign<&RoaringBitmap> for RoaringBitmap {
    /// A `symmetric difference` between two sets.
    fn bitxor_assign(&mut self, rhs: &RoaringBitmap) {
        self.xor_assign_cur(rhs)
    }
}
