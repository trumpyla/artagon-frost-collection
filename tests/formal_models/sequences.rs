//! `SEQ-MONOID` and the sequence projections from `docs/design/algebra/sequences.md`.

use super::common::words;

type Word = Vec<char>;

fn concat(s: &Word, t: &Word) -> Word {
    s.iter().chain(t).copied().collect()
}

fn reverse(s: &Word) -> Word {
    s.iter().rev().copied().collect()
}

fn counts(s: &Word) -> [usize; 2] {
    [
        s.iter().filter(|&&c| c == 'a').count(),
        s.iter().filter(|&&c| c == 'b').count(),
    ]
}

fn zip(s: &Word, t: &Word) -> Vec<(char, char)> {
    s.iter().copied().zip(t.iter().copied()).collect()
}

/// `SEQ-MONOID` and the homomorphisms of map, filter, `flat_map`, reverse, and
/// occurrence counting, over every word of at most three letters.
#[test]
fn concatenation_is_a_monoid_and_projections_respect_it() {
    let all = words(&['a', 'b'], 3);
    let upper = |c: char| c.to_ascii_uppercase();
    let double = |c: char| vec![c, c];
    for s in &all {
        assert_eq!(concat(s, &Vec::new()), *s);
        assert_eq!(concat(&Vec::new(), s), *s);
        for t in &all {
            let st = concat(s, t);
            assert_eq!(reverse(&st), concat(&reverse(t), &reverse(s)));
            let [sa, sb] = counts(s);
            let [ta, tb] = counts(t);
            assert_eq!(counts(&st), [sa + ta, sb + tb]);
            let map = |w: &Word| w.iter().copied().map(upper).collect::<Word>();
            assert_eq!(map(&st), concat(&map(s), &map(t)));
            let filter = |w: &Word| w.iter().copied().filter(|&c| c == 'a').collect::<Word>();
            assert_eq!(filter(&st), concat(&filter(s), &filter(t)));
            let flat = |w: &Word| w.iter().copied().flat_map(double).collect::<Word>();
            assert_eq!(flat(&st), concat(&flat(s), &flat(t)));
            for u in &all {
                assert_eq!(concat(&concat(s, t), u), concat(s, &concat(t, u)));
            }
        }
    }
    assert_ne!(
        concat(&vec!['a'], &vec!['b']),
        concat(&vec!['b'], &vec!['a'])
    );
}

/// Zip distributes over concatenation only when the left parts have equal length.
#[test]
fn zip_distributes_over_concatenation_only_when_aligned() {
    let all = words(&['a', 'b'], 3);
    for s in &all {
        for t in &all {
            for u in all.iter().filter(|u| u.len() == s.len()) {
                for v in &all {
                    assert_eq!(
                        zip(&concat(s, t), &concat(u, v)),
                        [zip(s, u), zip(t, v)].concat()
                    );
                }
            }
        }
    }
    let (s, t, u, v) = (vec!['a'], vec!['b'], Vec::new(), vec!['c']);
    assert_eq!(zip(&concat(&s, &t), &concat(&u, &v)), vec![('a', 'c')]);
    assert_eq!([zip(&s, &u), zip(&t, &v)].concat(), vec![('b', 'c')]);
}

/// Two readings of "list intersection" differ on duplicates.
#[test]
fn list_intersection_is_ambiguous_until_named() {
    let (left, right) = (vec!['a', 'a'], vec!['a']);
    let membership: Word = left.iter().copied().filter(|c| right.contains(c)).collect();
    let mut pool = right.clone();
    let multiset: Word = left
        .iter()
        .copied()
        .filter(|c| {
            pool.iter()
                .position(|x| x == c)
                .map(|i| pool.remove(i))
                .is_some()
        })
        .collect();
    assert_eq!((membership, multiset), (vec!['a', 'a'], vec!['a']));
}
