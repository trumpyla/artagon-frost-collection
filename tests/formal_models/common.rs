//! Finite domains shared by the models. Sets over at most eight keys are bit
//! masks: bit `k` is set when key `k` is a member.

/// Every subset of `n` keys.
pub(crate) fn subsets(n: u32) -> impl Iterator<Item = u32> + Clone {
    0..(1_u32 << n)
}

/// Members of `set` among the first `n` keys, ascending.
pub(crate) fn members(set: u32, n: u32) -> impl Iterator<Item = u32> {
    (0..n).filter(move |key| set & (1 << key) != 0)
}

/// Whether every member of `a` is a member of `b`.
pub(crate) fn is_subset(a: u32, b: u32) -> bool {
    a & !b == 0
}

/// Every word of exactly `len` symbols drawn from `0..base`.
pub(crate) fn tuples(base: u32, len: usize) -> Vec<Vec<u32>> {
    let mut words = vec![Vec::new()];
    for _ in 0..len {
        words = words
            .into_iter()
            .flat_map(|word| {
                (0..base).map(move |symbol| {
                    let mut next = word.clone();
                    next.push(symbol);
                    next
                })
            })
            .collect();
    }
    words
}

/// Every word of at most `max_len` symbols drawn from `alphabet`.
pub(crate) fn words<T: Copy>(alphabet: &[T], max_len: usize) -> Vec<Vec<T>> {
    let mut all = vec![Vec::new()];
    let mut level = vec![Vec::new()];
    for _ in 0..max_len {
        level = level
            .iter()
            .flat_map(|word: &Vec<T>| {
                alphabet.iter().map(move |&symbol| {
                    let mut next = word.clone();
                    next.push(symbol);
                    next
                })
            })
            .collect();
        all.extend(level.iter().cloned());
    }
    all
}
