//! Explicit runtime profiles within each collection's semantic family.
use frost::adaptive::{HashRepr, OrderedRepr};
use frost::prelude::*;

fn main() {
    let mut values: InlineVec<String, 2> = ["a", "b"].into_iter().map(String::from).collect();
    assert!(values.is_inline());
    values.push(String::from("c"));
    assert!(!values.is_inline());
    values.pop();
    assert!(!values.is_inline());
    values.shrink_to_fit();
    assert!(values.is_inline());
    let values = values.freeze().thaw();
    assert_eq!(values.as_slice(), &[String::from("a"), String::from("b")]);

    let mut counts = AdaptiveHashMap::with_policy(AdaptiveHash::linear_up_to(2));
    for key in ["b", "a", "c"] {
        counts.insert(key, 1);
    }
    assert_eq!(counts.representation(), HashRepr::Table);
    let original = counts.freeze();
    assert_eq!(original.get("b"), Some(&1));
    let mut counts = original.thaw_with(AdaptiveHash::linear_up_to(2));
    counts.remove("c");
    counts.shrink_to_fit();
    assert_eq!(counts.representation(), HashRepr::Linear);

    let mut ordered = AdaptiveOrderedMap::with_policy(AdaptiveOrdered::sorted_up_to(2));
    ordered.extend([(3, "c"), (1, "a"), (2, "b")]);
    assert_eq!(ordered.representation(), OrderedRepr::Tree);
    let frozen: FrozenOrderedMap<_, _> = ordered.freeze();
    assert_eq!(frozen.keys().copied().collect::<Vec<_>>(), [1, 2, 3]);
    let restored = AdaptiveOrderedMap::from_frozen(frozen, AdaptiveOrdered::sorted_up_to(4));
    assert_eq!(restored.representation(), OrderedRepr::Sorted);
}

#[cfg(test)]
mod tests {
    #[test]
    fn runs_adaptive_example() {
        super::main();
    }
}
