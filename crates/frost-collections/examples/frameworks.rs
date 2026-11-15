//! Serialize, validate, and process collection families with optional frameworks.

use frost::backend::{arbitrary, borsh, rayon, schemars};
use frost::prelude::*;

fn main() {
    use arbitrary::{Arbitrary, Unstructured};
    use rayon::prelude::*;

    let catalog: FrozenOrderedMap<String, u32> =
        serde_json::from_str(r#"{"pending":3,"complete":7}"#).unwrap();
    assert_eq!(
        catalog.keys().map(String::as_str).collect::<Vec<_>>(),
        ["complete", "pending"]
    );
    assert_eq!(catalog.par_iter().map(|(_, count)| *count).sum::<u32>(), 10);

    let bytes = borsh::to_vec(&catalog).unwrap();
    let recovered: FrozenOrderedMap<String, u32> = borsh::from_slice(&bytes).unwrap();
    assert_eq!(recovered, catalog);

    let schema = schemars::schema_for!(FrozenOrderedMap<String, u32>);
    let native_schema = schemars::schema_for!(std::collections::BTreeMap<String, u32>);
    assert_eq!(schema, native_schema);

    let input = [0_u8; 32];
    let generated = InlineVec::<u8, 4>::arbitrary(&mut Unstructured::new(&input)).unwrap();
    assert_eq!(generated.as_slice(), generated.clone().freeze().as_slice());

    let totals: FrozenOrderedMap<u32, u32> = (0..20_u32)
        .into_par_iter()
        .map(|value| (value % 2, value))
        .collect();
    assert_eq!(totals.get(&0), Some(&18));
    assert_eq!(totals.get(&1), Some(&19));
}

#[cfg(test)]
mod tests {
    #[test]
    fn executes_all_framework_scenarios() {
        super::main();
    }
}
