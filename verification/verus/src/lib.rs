//! Spec-level Verus proofs of collection-algebra laws from `docs/design/algebra`.
//!
//! These prove the mathematics over vstd's finite `Set`, `Map` and `Seq` and over
//! natural-number counts. Nothing here links to Frost: a proof here says the law
//! holds for the reference definition, not that any Frost type implements it.
//! The tie to real code is the Kani refinement harnesses in `verification/kani`,
//! within their bounds, and the finite models in `tests/formal_models`.
//!
//! Run with `make verus`. Law IDs refer to `docs/design/algebra/catalog.json`.
#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------- SET-BOOLEAN

pub open spec fn xor<A>(a: Set<A>, b: Set<A>) -> Set<A> {
    a.difference(b).union(b.difference(a))
}

/// `SET-BOOLEAN`: union and intersection are associative and commutative.
pub proof fn set_union_and_intersection_form_semilattices<A>(a: Set<A>, b: Set<A>, c: Set<A>)
    ensures
        a.union(b).union(c) == a.union(b.union(c)),
        a.intersect(b).intersect(c) == a.intersect(b.intersect(c)),
        a.union(b) == b.union(a),
        a.intersect(b) == b.intersect(a),
        a.union(a) == a,
        a.intersect(a) == a,
{
    assert(a.union(b).union(c) =~= a.union(b.union(c)));
    assert(a.intersect(b).intersect(c) =~= a.intersect(b.intersect(c)));
    assert(a.union(b) =~= b.union(a));
    assert(a.intersect(b) =~= b.intersect(a));
    assert(a.union(a) =~= a);
    assert(a.intersect(a) =~= a);
}

/// `SET-BOOLEAN`: absorption and both distributive laws.
pub proof fn set_lattice_is_distributive<A>(a: Set<A>, b: Set<A>, c: Set<A>)
    ensures
        a.union(a.intersect(b)) == a,
        a.intersect(a.union(b)) == a,
        a.intersect(b.union(c)) == a.intersect(b).union(a.intersect(c)),
        a.union(b.intersect(c)) == a.union(b).intersect(a.union(c)),
{
    assert(a.union(a.intersect(b)) =~= a);
    assert(a.intersect(a.union(b)) =~= a);
    assert(a.intersect(b.union(c)) =~= a.intersect(b).union(a.intersect(c)));
    assert(a.union(b.intersect(c)) =~= a.union(b).intersect(a.union(c)));
}

/// `SET-BOOLEAN`: symmetric difference is associative and self-cancelling.
pub proof fn symmetric_difference_is_a_group_operation<A>(a: Set<A>, b: Set<A>, c: Set<A>)
    ensures
        xor(xor(a, b), c) == xor(a, xor(b, c)),
        xor(a, a) == Set::<A>::empty(),
        xor(a, b) == xor(b, a),
{
    assert(xor(xor(a, b), c) =~= xor(a, xor(b, c)));
    assert(xor(a, a) =~= Set::<A>::empty());
    assert(xor(a, b) =~= xor(b, a));
}

// -------------------------------------------------------------------- SET-REP

/// A set with stored representatives: each key maps to the stored object.
/// Union keeps the left operand's object; intersection keeps the left's
/// object for keys the right also holds.
pub open spec fn union_left<K, V>(a: Map<K, V>, b: Map<K, V>) -> Map<K, V> {
    b.union_prefer_right(a)
}

pub open spec fn intersect_left<K, V>(a: Map<K, V>, b: Map<K, V>) -> Map<K, V> {
    a.restrict(b.dom())
}

/// `SET-REP`: left-representative union and intersection associate, and union
/// selects the left object for every key the left operand holds.
pub proof fn left_representatives_associate<K, V>(a: Map<K, V>, b: Map<K, V>, c: Map<K, V>)
    ensures
        union_left(union_left(a, b), c) == union_left(a, union_left(b, c)),
        intersect_left(intersect_left(a, b), c) == intersect_left(a, intersect_left(b, c)),
        forall|k: K| #[trigger] a.dom().contains(k) ==> union_left(a, b)[k] == a[k],
        forall|k: K| #[trigger] intersect_left(a, b).dom().contains(k) ==> intersect_left(a, b)[k] == a[k],
{
    assert(union_left(union_left(a, b), c) =~= union_left(a, union_left(b, c)));
    assert(intersect_left(intersect_left(a, b), c) =~= intersect_left(a, intersect_left(b, c)));
}

// ------------------------------------------------------------------ DOM-IMAGE

/// Preimage of `t` under `f`, within the finite domain `d`.
pub open spec fn preimage<A, B>(d: Set<A>, f: spec_fn(A) -> B, t: Set<B>) -> Set<A> {
    d.filter(|x: A| t.contains(f(x)))
}

/// `DOM-IMAGE`: direct image preserves union for every conversion, and maps an
/// intersection into the intersection of the images.
pub proof fn image_preserves_union_and_bounds_intersection<A, B>(
    a: Set<A>,
    b: Set<A>,
    f: spec_fn(A) -> B,
)
    ensures
        a.union(b).map(f) == a.map(f).union(b.map(f)),
        a.intersect(b).map(f).subset_of(a.map(f).intersect(b.map(f))),
{
    assert forall|y: B| #[trigger] a.union(b).map(f).contains(y) <==> a.map(f).union(
        b.map(f),
    ).contains(y) by {
        if a.union(b).map(f).contains(y) {
            let x = choose|x: A| a.union(b).contains(x) && y == f(x);
            if a.contains(x) {
                assert(a.map(f).contains(y));
            } else {
                assert(b.map(f).contains(y));
            }
        }
        if a.map(f).contains(y) {
            let x = choose|x: A| a.contains(x) && y == f(x);
            assert(a.union(b).contains(x));
        }
        if b.map(f).contains(y) {
            let x = choose|x: A| b.contains(x) && y == f(x);
            assert(a.union(b).contains(x));
        }
    }
    assert(a.union(b).map(f) =~= a.map(f).union(b.map(f)));
    assert forall|y: B| #[trigger] a.intersect(b).map(f).contains(y) implies a.map(f).intersect(
        b.map(f),
    ).contains(y) by {
        let x = choose|x: A| a.intersect(b).contains(x) && y == f(x);
        assert(a.contains(x) && b.contains(x));
    }
}

/// `DOM-IMAGE`: preimage preserves union, intersection and difference.
pub proof fn preimage_preserves_the_boolean_operations<A, B>(
    d: Set<A>,
    f: spec_fn(A) -> B,
    s: Set<B>,
    t: Set<B>,
)
    ensures
        preimage(d, f, s.union(t)) == preimage(d, f, s).union(preimage(d, f, t)),
        preimage(d, f, s.intersect(t)) == preimage(d, f, s).intersect(preimage(d, f, t)),
        preimage(d, f, s.difference(t)) == preimage(d, f, s).difference(preimage(d, f, t)),
{
    assert(preimage(d, f, s.union(t)) =~= preimage(d, f, s).union(preimage(d, f, t)));
    assert(preimage(d, f, s.intersect(t)) =~= preimage(d, f, s).intersect(preimage(d, f, t)));
    assert(preimage(d, f, s.difference(t)) =~= preimage(d, f, s).difference(
        preimage(d, f, t),
    ));
}

// ------------------------------------------------------------------- BAG-LAWS

// Bag operations are pointwise on counts, so the bag laws are these scalar laws
// at every key.

pub open spec fn count_max(a: nat, b: nat) -> nat {
    if a >= b { a } else { b }
}

pub open spec fn count_min(a: nat, b: nat) -> nat {
    if a <= b { a } else { b }
}

pub open spec fn monus(a: nat, b: nat) -> nat {
    if a >= b { (a - b) as nat } else { 0 }
}

pub open spec fn abs_diff(a: nat, b: nat) -> nat {
    if a >= b { (a - b) as nat } else { (b - a) as nat }
}

/// `BAG-LAWS`: sum, max and min associate and commute, max and min distribute
/// over each other, and the identities that pin min, monus, max and absolute
/// difference to each other hold.
pub proof fn count_operations_satisfy_their_laws(a: nat, b: nat, c: nat)
    ensures
        (a + b) + c == a + (b + c),
        count_max(count_max(a, b), c) == count_max(a, count_max(b, c)),
        count_min(count_min(a, b), c) == count_min(a, count_min(b, c)),
        count_max(a, b) == count_max(b, a),
        count_min(a, b) == count_min(b, a),
        count_max(a, count_min(b, c)) == count_min(count_max(a, b), count_max(a, c)),
        count_min(a, count_max(b, c)) == count_max(count_min(a, b), count_min(a, c)),
        count_min(a, b) + monus(a, b) == a,
        count_max(a, b) == a + monus(b, a),
        abs_diff(a, b) == monus(a, b) + monus(b, a),
{
}

/// `BAG-SUPPORT`: a positive count of a sum or a max is a positive count in
/// either operand; of a min, in both.
pub proof fn support_follows_the_lattice(a: nat, b: nat)
    ensures
        (a + b > 0) == (a > 0 || b > 0),
        (count_max(a, b) > 0) == (a > 0 || b > 0),
        (count_min(a, b) > 0) == (a > 0 && b > 0),
{
}

// -------------------------------------------------------------- COUNT-PARTIAL

pub open spec fn checked_sum(bound: nat, a: Option<nat>, b: Option<nat>) -> Option<nat> {
    match (a, b) {
        (Some(x), Some(y)) => if x + y <= bound { Some((x + y) as nat) } else { None },
        _ => None,
    }
}

/// `COUNT-PARTIAL`: with counts bounded by `bound`, both groupings of a
/// checked sum succeed exactly when the total fits, and then agree.
pub proof fn checked_sum_associates(bound: nat, a: nat, b: nat, c: nat)
    ensures
        checked_sum(bound, checked_sum(bound, Some(a), Some(b)), Some(c)) == (if a + b + c
            <= bound {
            Some((a + b + c) as nat)
        } else {
            None::<nat>
        }),
        checked_sum(bound, Some(a), checked_sum(bound, Some(b), Some(c))) == (if a + b + c
            <= bound {
            Some((a + b + c) as nat)
        } else {
            None::<nat>
        }),
{
}

// ----------------------------------------------------------------- SEQ-MONOID

/// `SEQ-MONOID`: concatenation is associative with the empty sequence as unit.
pub proof fn concatenation_is_a_monoid<A>(s: Seq<A>, t: Seq<A>, u: Seq<A>)
    ensures
        s + t + u == s + (t + u),
        s + Seq::<A>::empty() == s,
        Seq::<A>::empty() + s == s,
{
    assert(s + t + u =~= s + (t + u));
    assert(s + Seq::<A>::empty() =~= s);
    assert(Seq::<A>::empty() + s =~= s);
}

// --------------------------------------------------------------- BIMAP-PARTIAL

/// A relation is a partial bijection when each side determines the other.
pub open spec fn is_pbij<A, B>(r: Set<(A, B)>) -> bool {
    forall|p: (A, B), q: (A, B)|
        #[trigger] r.contains(p) && #[trigger] r.contains(q) ==> (p.0 == q.0 <==> p.1 == q.1)
}

pub open spec fn checked_union<A, B>(r: Option<Set<(A, B)>>, s: Option<Set<(A, B)>>) -> Option<
    Set<(A, B)>,
> {
    match (r, s) {
        (Some(r), Some(s)) => if is_pbij(r.union(s)) { Some(r.union(s)) } else { None },
        _ => None,
    }
}

/// Every subset of a partial bijection is one.
pub proof fn partial_bijections_are_downward_closed<A, B>(s: Set<(A, B)>, r: Set<(A, B)>)
    requires
        s.subset_of(r),
        is_pbij(r),
    ensures
        is_pbij(s),
{
    assert forall|p: (A, B), q: (A, B)| s.contains(p) && s.contains(q) implies (p.0 == q.0
        <==> p.1 == q.1) by {
        assert(r.contains(p) && r.contains(q));
    }
}

/// `BIMAP-PARTIAL`: checked union succeeds in either grouping exactly when the
/// three-way union is a partial bijection, whatever the operands.
pub proof fn checked_union_succeeds_exactly_when_the_whole_union_is_valid<A, B>(
    r: Set<(A, B)>,
    s: Set<(A, B)>,
    t: Set<(A, B)>,
)
    ensures
        checked_union(checked_union(Some(r), Some(s)), Some(t)) == (if is_pbij(
            r.union(s).union(t),
        ) {
            Some(r.union(s).union(t))
        } else {
            None
        }),
        checked_union(Some(r), checked_union(Some(s), Some(t))) == (if is_pbij(
            r.union(s).union(t),
        ) {
            Some(r.union(s).union(t))
        } else {
            None
        }),
{
    let whole = r.union(s).union(t);
    assert(r.union(s.union(t)) =~= whole);
    if is_pbij(whole) {
        partial_bijections_are_downward_closed(r.union(s), whole);
        partial_bijections_are_downward_closed(s.union(t), whole);
    }
}

// ---------------------------------------------------------------- KNOW-BOUNDS

/// `x` lies between the bounds `lower` and `upper`.
pub open spec fn within<A>(x: Set<A>, lower: Set<A>, upper: Set<A>) -> bool {
    lower.subset_of(x) && x.subset_of(upper)
}

/// `KNOW-BOUNDS`: the transfer rules for union, intersection and difference are
/// sound for every realization of the operands.
pub proof fn knowledge_bounds_are_sound<A>(
    x: Set<A>,
    lx: Set<A>,
    ux: Set<A>,
    y: Set<A>,
    ly: Set<A>,
    uy: Set<A>,
)
    requires
        within(x, lx, ux),
        within(y, ly, uy),
    ensures
        within(x.union(y), lx.union(ly), ux.union(uy)),
        within(x.intersect(y), lx.intersect(ly), ux.intersect(uy)),
        within(x.difference(y), lx.difference(uy), ux.difference(ly)),
{
}

} // verus!
