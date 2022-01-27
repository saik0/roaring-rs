#[cfg(test)]
#[allow(clippy::eq_op)] // Allow equal expressions as operands
#[allow(clippy::redundant_clone)] // Allow equal expressions as operands
mod test {
    use crate::bitmap::store::ArrayStore;
    use proptest::prelude::*;

    //
    // Tests algebraic set properties in terms of RoaringBitmaps.
    // Follows wikipedia article regarding ordering and heading
    //
    // https://en.wikipedia.org/wiki/Algebra_of_sets
    //
    // Notes:
    //
    //  * Although a universe set exists, we leave properties involving it it out of these tests.
    //    It would be ~512 MiB and operations on it would be relatively slow
    //
    //  * Likewise, there is no compliment operator
    //
    //
    //
    //
    // The fundamental properties of set algebra
    // =========================================
    //
    // Commutative property:
    // --------------------

    proptest! {
        #[test]
        fn unions_are_commutative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(&a | &b, &b | &a);
        }

        #[test]
        fn intersections_are_commutative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(&a & &b, &b & &a);

            { // op_assign_ref
                let mut x = a.clone();
                let mut y = b.clone();

                x &= &b;
                y &= &a;

                prop_assert_eq!(x, y);
            }
        }

        #[test]
        fn symmetric_differences_are_commutative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(&a ^ &b, &b ^ &a);

        }
    }

    //
    // Associative property:
    // ---------------------

    proptest! {
        #[test]
        fn unions_are_associative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a | &( &b | &c ),
                &( &a | &b ) | &c
            );


        }

        #[test]
        fn intersections_are_associative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a & &( &b & &c ),
                &( &a & &b ) & &c
            );


        }

        #[test]
        fn symmetric_differences_are_associative(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a ^ &( &b ^ &c ),
                &( &a ^ &b ) ^ &c
            );


        }
    }

    //
    // Distributive property:
    // ---------------------

    proptest! {
        #[test]
        fn union_distributes_over_intersection(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a | &( &b & &c),
                &( &a | &b ) & &( &a | &c )
            );


        }

        #[test]
        fn intersection_distributes_over_union(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a & &( &b | &c),
                &( &a & &b ) | &( &a & &c )
            );


        }

        #[test]
        fn intersection_distributes_over_symmetric_difference(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &a & &( &b ^ &c),
                &( &a & &b ) ^ &( &a & &c )
            );


        }
    }

    // Identity:
    // --------

    proptest! {
        #[test]
        fn the_empty_set_is_the_identity_for_union(a in ArrayStore::arbitrary()) {
            prop_assert_eq!(&(&a | &empty_set()), &a);

        }

        #[test]
        fn the_empty_set_is_the_identity_for_symmetric_difference(a in ArrayStore::arbitrary()) {
            prop_assert_eq!(&(&a ^ &empty_set()), &a);


        }
    }

    // Some additional laws for unions and intersections
    // =================================================
    //
    // PROPOSITION 3: For any subsets A and B of a universe set U, the following identities hold:
    //
    // Idempotent laws
    // ---------------

    proptest! {
        #[test]
        fn unions_are_idempotent(a in ArrayStore::arbitrary()) {
            prop_assert_eq!(&(&a | &a), &a);


        }

        #[test]
        fn intersections_are_idempotent(a in ArrayStore::arbitrary()) {
            prop_assert_eq!(&(&a & &a), &a);


        }
    }

    //
    // Domination laws
    // ---------------

    proptest! {
        #[test]
        fn empty_set_domination(a in ArrayStore::arbitrary()) {
            prop_assert_eq!(&a & &empty_set(), empty_set());


        }
    }

    // The algebra of inclusion
    // ========================
    // PROPOSITION 6: If A, B and C are sets then the following hold:
    //
    // Note that for inclusion we do not also assert for the assignment operators
    // Inclusion is the property under test, not the set operation

    proptest! {
        #[test]
        fn reflexivity(a in ArrayStore::arbitrary()) {
            prop_assert!(a.is_subset(&a));
        }

        #[test]
        fn antisymmetry(a in ArrayStore::arbitrary(), b in ArrayStore::arbitrary()) {
            if a == b {
                prop_assert!(a.is_subset(&b) && b.is_subset(&a));
            } else {
                prop_assert!(!(a.is_subset(&b) && b.is_subset(&a)));
            }
        }

        #[test]
        fn transitivity(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            let b = &b | &a;
            let c  = &b | &c;
            // If
            prop_assert!(a.is_subset(&b));
            prop_assert!(b.is_subset(&c));
            // Then
            prop_assert!(a.is_subset(&c));

        }
    }

    // PROPOSITION 7: If A, B and C are subsets of a set S then the following hold:

    proptest! {
        #[test]
        fn existence_of_joins(a in ArrayStore::arbitrary(), b in ArrayStore::arbitrary()) {
            prop_assert!(a.is_subset(&(&a | &b)));
        }

        #[test]
        fn existence_of_meets(a in ArrayStore::arbitrary(), b in ArrayStore::arbitrary()) {
            prop_assert!(&(&a & &b).is_subset(&a));
        }
    }

    // PROPOSITION 8: For any two sets A and B, the following are equivalent:

    proptest! {
        #[test]
        fn inclusion_can_be_characterized_by_union_or_inersection(
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            let a = &b - &c;

            prop_assert!(a.is_subset(&b));
            prop_assert_eq!(&(&a & &b), &a);
            prop_assert_eq!(&(&a | &b), &b);
            prop_assert_eq!(&(&a - &b), &empty_set());
        }
    }

    // The algebra of relative complements
    // ===================================
    //
    // PROPOSITION 9: For any universe U and subsets A, B, and C of U,
    // the following identities hold:

    proptest! {
        #[test]
        fn relative_compliments(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            let u = &(&a | &b) | &c;

            prop_assert_eq!(
                &c - &(&a & &b),
                &(&c - &a) | &(&c - &b)
            );

            prop_assert_eq!(
                &c - &(&a | &b),
                &(&c - &a) & &(&c - &b)
            );

            prop_assert_eq!(
                &c - &(&b - &a),
                &(&a & &c) | &(&c - &b)
            );

            {
                let x = &(&b - &a) & &c;
                let y = &(&b & &c) - &a;
                let z = &b & &(&c - &a);

                prop_assert_eq!(&x, &y);
                prop_assert_eq!(&y, &z);
                prop_assert_eq!(&z, &x);
            }

            prop_assert_eq!(
                &(&b - &a) | &c,
                &(&b | &c) - &(&a - &c)
            );

            prop_assert_eq!(
                &(&b - &a) - &c,
                &b - &(&a | &c)
            );

            prop_assert_eq!(
                &a - &a,
                empty_set()
            );

             prop_assert_eq!(
                &empty_set() - &a,
                empty_set()
            );

            prop_assert_eq!(
                &a - &u,
                empty_set()
            );
        }
    }

    // Additional properties of symmetric differences
    // ==============================================
    //

    proptest! {
        #[test]
        fn symmetric_difference_triangle_inequality(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary(),
            c in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &(&(&a ^ &b) ^ &(&b ^ &c)),
                &(&a ^ &c)
            );
        }

        #[test]
        fn symmetric_difference_empty_set_neutral(
            a in ArrayStore::arbitrary()
        ) {
            prop_assert_eq!(
                &(&a ^ &empty_set()),
                &a
            );
        }

        #[test]
        fn symmetric_difference_inverse_of_itself(
            a in ArrayStore::arbitrary()
        ) {

            prop_assert_eq!(
                &(&a ^ &a),
                &empty_set()
            );
        }

        #[test]
        fn symmetric_difference_relative_compliments(
            a in ArrayStore::arbitrary(),
            b in ArrayStore::arbitrary()
        ) {

            prop_assert_eq!(
                &(&a ^ &b),
                &(&(&a - &b) | &(&b - &a))
            );

            prop_assert_eq!(
                &(&a ^ &b),
                &(&(&a | &b) - &(&a & &b))
            );
        }
    }

    fn empty_set() -> ArrayStore {
        ArrayStore::new()
    }
}
