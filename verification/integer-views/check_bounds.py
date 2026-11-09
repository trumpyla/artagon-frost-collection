"""Independent finite-domain oracle for proposed IntSetExpr cardinality bounds.

Research-only model: does not exercise production code.
"""
from itertools import product

if not __debug__:
    raise RuntimeError("Run without Python optimization; assertions perform the checks.")


def bounds(u, la, ua, lb, ub):
    assert 0 <= la <= ua <= u
    assert 0 <= lb <= ub <= u
    return (
        (max(la, lb), min(u, ua + ub)),
        (max(0, la + lb - u), min(ua, ub)),
        (max(0, la - ub), min(ua, u - lb)),
        (max(0, la - ub, lb - ua), min(ua + ub, 2 * u - la - lb, u)),
    )


pair_cases = interval_cases = disjoint_cases = disjoint_interval_cases = 0
for u in range(7):
    intervals = [(lo, hi) for lo in range(u + 1) for hi in range(lo, u + 1)]
    formulas = {(ia, ib): bounds(u, *ia, *ib) for ia in intervals for ib in intervals}
    observed = {key: [[u + 1, -1] for _ in range(4)] for key in formulas}
    enclosing = {n: [i for i in intervals if i[0] <= n <= i[1]] for n in range(u + 1)}
    for a, b in product(range(1 << u), repeat=2):
        counts = ((a | b).bit_count(), (a & b).bit_count(), (a & ~b).bit_count(), (a ^ b).bit_count())
        na, nb = a.bit_count(), b.bit_count()
        pair_cases += 1
        disjoint = False
        if a and b:
            amin = (a & -a).bit_length() - 1
            bmin = (b & -b).bit_length() - 1
            amax = a.bit_length() - 1
            bmax = b.bit_length() - 1
            disjoint = amax < bmin or bmax < amin
        if disjoint:
            assert counts == (na + nb, 0, na, na + nb)
            disjoint_cases += 1
        for ia, ib in product(enclosing[na], enclosing[nb]):
            bs = formulas[ia, ib]
            seen = observed[ia, ib]
            for j, (actual, (lo, hi)) in enumerate(zip(counts, bs)):
                assert 0 <= lo <= actual <= hi <= u, (u, a, b, ia, ib, j, actual, bs)
                seen[j][0] = min(seen[j][0], actual)
                seen[j][1] = max(seen[j][1], actual)
            interval_cases += 1
            if disjoint:
                # Loose upper bounds can sum above U despite disjoint support.
                added = (ia[0] + ib[0], min(u, ia[1] + ib[1]))
                refined = (added, (0, 0), ia, added)
                for j, (actual, (lo, hi)) in enumerate(zip(counts, refined)):
                    assert 0 <= lo <= actual <= hi <= u, (u, a, b, ia, ib, j, actual, refined)
                disjoint_interval_cases += 1
    for key, actual_bounds in observed.items():
        assert tuple(map(tuple, actual_bounds)) == formulas[key], (u, key, actual_bounds, formulas[key])
    print(f'U={u}: sound and individually tight for every enclosing interval pair')

wide_cases = 0
for u in (1 << 32, 1 << 64):
    points = (0, 1, u // 2 - 1, u // 2, u // 2 + 1, u - 1, u)
    intervals = [(lo, hi) for lo in points for hi in points if lo <= hi]
    for ia, ib in product(intervals, repeat=2):
        bs = bounds(u, *ia, *ib)
        assert all(0 <= lo <= hi <= u for lo, hi in bs)
        # Every intermediate of the candidate formulas fits u128.
        assert max(ia[1] + ib[1], 2 * u, ia[0] + ib[0]) < (1 << 128)
        for na, nb in product(ia, ib):
            # Realizable extreme intersection sizes for these actual counts.
            for common in (max(0, na + nb - u), min(na, nb)):
                actuals = (na + nb - common, common, na - common, na + nb - 2 * common)
                assert all(lo <= actual <= hi for actual, (lo, hi) in zip(actuals, bs))
        wide_cases += 1
print({'subset_pairs': pair_cases, 'subset_and_enclosing_interval_cases': interval_cases,
       'disjoint_support_pairs': disjoint_cases, 'disjoint_interval_cases': disjoint_interval_cases,
       'wide_interval_cases': wide_cases})
