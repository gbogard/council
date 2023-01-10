use super::views::*;

#[quickcheck]
fn cluster_view_hash_is_deterministic(view: ClusterView) -> bool {
    view.hash() == view.hash()
}

/// Tests that for all states a, b and c, merging b into a and then c into a is equivalent to merging c into b and then b into a, i.e.
/// that the merge function is associative. This is the first important property of state-based CRDT. Commutativity and idempotence are
/// tested below.
///
/// We could express associativity as "for all states a, b and c, merge(merge(a, b), c) = merge(a, merge(b, c))", except that here, we mutate
/// state rather than returning an immutable state so "merge" is not quite exactly a function in the mathemtical sense.
#[quickcheck]
fn merge_test_associativity(mut a: ClusterView, mut b: ClusterView, c: ClusterView) -> bool {
    let merged_a_and_b_first = {
        let mut res = a.clone();
        res.merge(b.clone());
        res.merge(c.clone());
        res
    };
    let merged_b_and_c_first = {
        b.merge(c);
        a.merge(b);
        a
    };
    merged_a_and_b_first == merged_b_and_c_first
}

/// Tests that for all states a and b, merging a into b is equivalent to merging b into a, i.e. that merging is commutative.
/// Again, this could be expressed as "merge(a, b) = merge(b, a) for all states a and b", except that we use mutation rather then immutable states.
#[quickcheck]
fn merge_test_commutativity(a: ClusterView, mut b: ClusterView) -> bool {
    let merged_a_b = {
        let mut a = a.clone();
        a.merge(b.clone());
        a
    };
    let merged_b_a = {
        b.merge(a);
        b
    };
    merged_a_b == merged_b_a
}

/// Tests that for all states a and b, once we have merged b into a, merging b again into the resulting state is a no-op,
/// i.e. that merging is idempotent.
#[quickcheck]
fn merge_test_idempotence(mut a: ClusterView, b: ClusterView) -> bool {
    let merged_a_b = {
        a.merge(b.clone());
        a
    };
    let merged_a_b_b = {
        let mut merged_a_b = merged_a_b.clone();
        merged_a_b.merge(b);
        merged_a_b
    };
    merged_a_b == merged_a_b_b
}
