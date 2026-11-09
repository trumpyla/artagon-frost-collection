#!/usr/bin/env python3
"""Finite design model; deliberately imports no Frost implementation.

A set maps a logical key to one stable source-labelled representative. Equality
and hashing depend only on the key. Algebra is pure and observes fixed operand
membership. Union/intersection retain the left representative; difference and
XOR retain the surviving representative. Enumeration order is treated separately
from the key-to-representative denotation.

The model checks rewrite laws over all one-key presence combinations and checks
a proposed recursive borrowed enumerator over all ordered subsets of two keys,
six source permutations, two binary shapes, and four operations at each node.
It establishes only these finite model claims, not Rust lifetime/ownership
correctness, a Frost API implementation, or performance measurements.
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import permutations, product
import json
from pathlib import Path


OPS = ("union", "intersection", "difference", "xor")
NAMES = ("A", "B", "C")
KEYS = ("x", "y")


@dataclass(frozen=True)
class Expr:
    op: str
    left: "Expr | None" = None
    right: "Expr | None" = None

    def __str__(self) -> str:
        if self.op in NAMES:
            return self.op
        symbol = {"union": "|", "intersection": "&", "difference": "-", "xor": "^"}[self.op]
        return f"({self.left} {symbol} {self.right})"


A, B, C = (Expr(name) for name in NAMES)


def node(op: str, left: Expr, right: Expr) -> Expr:
    return Expr(op, left, right)


def reference(expr: Expr, env: dict[str, dict[str, str]]) -> dict[str, str]:
    """Independent denotation: explicit key-set operations, then provenance."""
    if expr.op in NAMES:
        return dict(env[expr.op])
    left = reference(expr.left, env)
    right = reference(expr.right, env)
    lhs, rhs = set(left), set(right)
    if expr.op == "union":
        keys = lhs | rhs
    elif expr.op == "intersection":
        keys = lhs & rhs
    elif expr.op == "difference":
        keys = lhs - rhs
    else:
        keys = lhs ^ rhs
    return {key: left[key] if key in left else right[key] for key in keys}


def contains(expr: Expr, key: str, env: dict[str, dict[str, str]]) -> bool:
    if expr.op in NAMES:
        return key in env[expr.op]
    lhs = contains(expr.left, key, env)
    if expr.op == "union":
        return lhs or contains(expr.right, key, env)
    if expr.op == "intersection":
        return lhs and contains(expr.right, key, env)
    if expr.op == "difference":
        return lhs and not contains(expr.right, key, env)
    return lhs != contains(expr.right, key, env)


def enumerate_recursive(expr: Expr, env: dict[str, dict[str, str]]):
    """Proposed baseline algorithm; never materializes intermediate sets."""
    if expr.op in NAMES:
        yield from env[expr.op].items()
        return
    if expr.op == "intersection":
        for key, rep in enumerate_recursive(expr.left, env):
            if contains(expr.right, key, env):
                yield key, rep
    elif expr.op == "difference":
        for key, rep in enumerate_recursive(expr.left, env):
            if not contains(expr.right, key, env):
                yield key, rep
    elif expr.op == "union":
        yield from enumerate_recursive(expr.left, env)
        for key, rep in enumerate_recursive(expr.right, env):
            if not contains(expr.left, key, env):
                yield key, rep
    else:
        for key, rep in enumerate_recursive(expr.left, env):
            if not contains(expr.right, key, env):
                yield key, rep
        for key, rep in enumerate_recursive(expr.right, env):
            if not contains(expr.left, key, env):
                yield key, rep


def presence_env(bits: tuple[bool, ...]) -> dict[str, dict[str, str]]:
    return {name: {"x": f"{name}:x"} if present else {} for name, present in zip(NAMES, bits)}


def check_rewrite(name: str, left: Expr, right: Expr) -> dict:
    cases = []
    for bits in product((False, True), repeat=3):
        env = presence_env(bits)
        lhs, rhs = reference(left, env), reference(right, env)
        cases.append({
            "presence": dict(zip(NAMES, bits)),
            "left": lhs,
            "right": rhs,
            "membership_equal": lhs.keys() == rhs.keys(),
            "representatives_equal": lhs == rhs,
            "enumeration_equal": list(enumerate_recursive(left, env)) == list(enumerate_recursive(right, env)),
        })
    order_differences = 0
    order_only_example = None
    for env in ordered_inputs():
        lhs = list(enumerate_recursive(left, env))
        rhs = list(enumerate_recursive(right, env))
        if lhs != rhs:
            order_differences += 1
            if dict(lhs) == dict(rhs) and order_only_example is None:
                order_only_example = {"input": env, "left": lhs, "right": rhs}
    return {
        "name": name,
        "left_expression": str(left),
        "right_expression": str(right),
        "cases_checked": len(cases),
        "membership_equivalent": all(case["membership_equal"] for case in cases),
        "representative_equivalent": all(case["representatives_equal"] for case in cases),
        "counterexamples": [case for case in cases if not case["representatives_equal"]],
        "two_key_order_cases_checked": 125,
        "enumeration_mismatches_including_membership_and_provenance": order_differences,
        "order_only_counterexample": order_only_example,
    }


def ordered_inputs():
    # Empty, either singleton, and both orders of the two-member set.
    possibilities = [(), ("x",), ("y",), ("x", "y"), ("y", "x")]
    for source_orders in product(possibilities, repeat=3):
        yield {
            name: {key: f"{name}:{key}" for key in order}
            for name, order in zip(NAMES, source_orders)
        }


def shape_cases():
    for left, middle, right in permutations((A, B, C)):
        for inner, outer in product(OPS, repeat=2):
            yield node(outer, node(inner, left, middle), right)
            yield node(outer, left, node(inner, middle, right))


def validate_enumerator() -> dict:
    count = 0
    shapes = tuple(shape_cases())
    for env in ordered_inputs():
        for expr in shapes:
            expected = reference(expr, env)
            actual = list(enumerate_recursive(expr, env))
            assert len(actual) == len(dict(actual)), (str(expr), env, actual, "duplicate")
            assert dict(actual) == expected, (str(expr), env, actual, expected)
            for key in KEYS:
                assert contains(expr, key, env) == (key in expected)
            count += 1
    return {
        "status": "passed",
        "keys": list(KEYS),
        "input_orders_per_source": 5,
        "source_configurations": 125,
        "expression_shapes_including_permutations": len(shapes),
        "expression_input_cases": count,
        "checked": ["membership", "representative provenance", "no duplicate output", "contains consistency"],
    }


def validate_associative_enumeration_order() -> dict:
    results = {}
    for op in ("union", "intersection"):
        count = 0
        for env in ordered_inputs():
            for left, middle, right in permutations((A, B, C)):
                lhs = node(op, node(op, left, middle), right)
                rhs = node(op, left, node(op, middle, right))
                assert list(enumerate_recursive(lhs, env)) == list(enumerate_recursive(rhs, env))
                count += 1
        results[op] = {"status": "passed", "cases": count, "source_order_permutation_retained_on_both_sides": True}
    return results


def candidate_cover_example() -> dict:
    env = {"A": {"x": "A:x", "y": "A:y"}, "B": {"y": "B:y"}, "C": {}}
    expr = node("intersection", A, B)
    candidates = list(env["B"].items())  # smaller operand; complete intersection cover
    accepted = [(key, rep) for key, rep in candidates if contains(expr, key, env)]
    expected = reference(expr, env)
    assert dict(accepted).keys() == expected.keys()
    assert dict(accepted) != expected
    return {
        "expression": str(expr),
        "input": env,
        "chosen_cover": "B (1 candidate, versus A with 2)",
        "candidate_output": dict(accepted),
        "required_output": expected,
        "membership_correct": True,
        "representative_correct": False,
        "explanation": "The cover supplies candidate keys, not authority to choose representatives. Resolve the representative against the original expression.",
    }


def order_counterexample() -> dict:
    env = {"A": {"y": "A:y"}, "B": {"x": "B:x"}, "C": {}}
    expr = node("union", A, B)
    actual = list(enumerate_recursive(expr, env))
    assert [key for key, _ in actual] == ["y", "x"]
    return {"expression": str(expr), "input_each_individually_sorted": True, "output": actual,
            "output_sorted": False,
            "explanation": "Left-then-right-only generic union preserves source encounter order, not sorted order across operands."}


def main() -> None:
    rewrites = []
    for op in ("union", "intersection", "xor"):
        rewrites.append(check_rewrite(f"{op}_associativity", node(op, node(op, A, B), C), node(op, A, node(op, B, C))))
    for op in OPS:
        rewrites.append(check_rewrite(f"{op}_commutativity", node(op, A, B), node(op, B, A)))
    rewrites.append(check_rewrite(
        "nonlocal_xor_cancellation",
        node("xor", node("xor", A, B), A),
        B,
    ))
    rewrites.append(check_rewrite(
        "right_union_distribution_over_intersection",
        node("union", node("intersection", A, B), C),
        node("intersection", node("union", A, C), node("union", B, C)),
    ))
    rewrites.append(check_rewrite(
        "left_intersection_distribution_over_union",
        node("intersection", A, node("union", B, C)),
        node("union", node("intersection", A, B), node("intersection", A, C)),
    ))
    rewrites.append(check_rewrite(
        "left_union_distribution_over_intersection",
        node("union", A, node("intersection", B, C)),
        node("intersection", node("union", A, B), node("union", A, C)),
    ))
    by_name = {item["name"]: item for item in rewrites}
    assert by_name["union_associativity"]["representative_equivalent"]
    assert by_name["intersection_associativity"]["representative_equivalent"]
    assert by_name["xor_associativity"]["membership_equivalent"]
    assert not by_name["xor_associativity"]["representative_equivalent"]
    assert not by_name["union_commutativity"]["representative_equivalent"]
    assert not by_name["intersection_commutativity"]["representative_equivalent"]
    assert by_name["xor_commutativity"]["representative_equivalent"]
    assert by_name["nonlocal_xor_cancellation"]["membership_equivalent"]
    assert not by_name["nonlocal_xor_cancellation"]["representative_equivalent"]
    assert not by_name["difference_commutativity"]["membership_equivalent"]
    assert not by_name["right_union_distribution_over_intersection"]["representative_equivalent"]
    results = {
        "status": "passed",
        "scope": "Finite independent design model; not production Frost validation or a Rust soundness proof.",
        "assumptions": [
            "Sets contain unique keys and fixed membership during evaluation.",
            "Equality and hashing depend only on key; representative labels remain observable.",
            "Union and intersection are left-biased; difference and XOR retain surviving representatives.",
            "Element lookups are pure and agree with iteration.",
            "Generic enumeration has source encounter order; sorted traversal is a separate kernel contract.",
        ],
        "rewrite_tests": rewrites,
        "recursive_enumerator": validate_enumerator(),
        "associative_enumeration_order": validate_associative_enumeration_order(),
        "candidate_cover_counterexample": candidate_cover_example(),
        "sorted_order_counterexample": order_counterexample(),
    }
    destination = Path(__file__).with_name("results.json")
    destination.write_text(json.dumps(results, indent=2, sort_keys=True) + "\n")
    print(json.dumps({
        "status": results["status"],
        "rewrite_laws": len(rewrites),
        "rewrite_presence_cases": len(rewrites) * 8,
        "rewrite_two_key_order_cases": len(rewrites) * 125,
        "recursive_enumerator_cases": results["recursive_enumerator"]["expression_input_cases"],
        "associative_order_cases": sum(item["cases"] for item in results["associative_enumeration_order"].values()),
        "results": str(destination),
    }, indent=2))


if __name__ == "__main__":
    main()
