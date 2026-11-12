/** Finite checks for the laws added with the prior-art survey of 2026-11-12.
 * Independent finite mathematics, not Frost execution, Rust verification or an unbounded proof.
 * Run: node docs/research/formal-models/evidence/prior-art-checks.mjs
 * No dependencies, network, clock, or repository writes. Separate from model-checks.mjs:
 * its counts are not part of that model's totals and model-mutants.mjs does not mutate it.
 * A "computed" counterexample evaluates the functions below on both sides.
 */
import assert from 'node:assert/strict';
const groups = [];
const counters = [];
let group;
function section(name, bounds, fn) {
  group = { name, bounds, assertions: 0, cases: 0 };
  groups.push(group);
  fn();
}
function check(value, message) { group.assertions++; assert.ok(value, message); }
function equal(a, b, message) { group.assertions++; assert.deepEqual(a, b, message); }
function oneCase() { group.cases++; }
function counter(id, left, right, why, at) {
  group.assertions++; assert.notDeepEqual(left, right, id);
  counters.push({ id, kind: 'computed', at, left, right, why });
}
const range = n => Array.from({ length: n }, (_, i) => i);
const monus = (a, b) => Math.max(a - b, 0);
/** First triple in lexicographic order where the two sides differ, or null. */
function firstDifference(n, lhs, rhs) {
  for (const a of range(n)) for (const b of range(n)) for (const c of range(n))
    if (lhs(a, b, c) !== rhs(a, b, c)) return { a, b, c, left: lhs(a, b, c), right: rhs(a, b, c) };
  return null;
}

section('BAG-MONUS', 'counts 0..6, all triples; subsets of three keys, all triples', () => {
  const N = 7;
  const holds = {
    'sum then monus cancels: (a+b) monus b = a': [(a, b) => monus(a + b, b), a => a],
    'iterated monus: a monus (b+c) = (a monus b) monus c': [(a, b, c) => monus(a, b + c), (a, b, c) => monus(monus(a, b), c)],
    'monus then sum is max: (a monus b)+b = max(a,b)': [(a, b) => monus(a, b) + b, (a, b) => Math.max(a, b)],
    'split by min: (a monus b)+min(a,b) = a': [(a, b) => monus(a, b) + Math.min(a, b), a => a],
    'double monus is min: a monus (a monus b) = min(a,b)': [(a, b) => monus(a, monus(a, b)), (a, b) => Math.min(a, b)],
    'monus distributes over max on the left': [(a, b, c) => monus(Math.max(a, b), c), (a, b, c) => Math.max(monus(a, c), monus(b, c))],
    'monus distributes over min on the left': [(a, b, c) => monus(Math.min(a, b), c), (a, b, c) => Math.min(monus(a, c), monus(b, c))],
    'monus by max is min of monus': [(a, b, c) => monus(a, Math.max(b, c)), (a, b, c) => Math.min(monus(a, b), monus(a, c))],
    'monus by min is max of monus': [(a, b, c) => monus(a, Math.min(b, c)), (a, b, c) => Math.max(monus(a, b), monus(a, c))],
    'product distributes over monus in N (axiom A5)': [(a, b, c) => monus(b, a) * c, (a, b, c) => monus(b * c, a * c)],
  };
  for (const [name, [lhs, rhs]] of Object.entries(holds))
    for (const a of range(N)) for (const b of range(N)) for (const c of range(N)) {
      oneCase(); equal(lhs(a, b, c), rhs(a, b, c), name);
    }
  // Identities of set difference that fail when union is read as bag sum and
  // intersection as min. Each set identity is first checked on all subsets.
  const U = 8; // bitsets over three keys
  const diff = (x, y) => x & ~y;
  const setForms = {
    'CX-BAG-MONUS-SUM-CANCEL': [(A, B) => diff(A | B, B), (A, B) => diff(A, B)],
    'CX-BAG-MONUS-SUM-DIST': [(A, B, C) => diff(A | B, C), (A, B, C) => diff(A, C) | diff(B, C)],
    'CX-BAG-MONUS-MIN': [(A, B, C) => A & diff(B, C), (A, B, C) => diff(A & B, C)],
    'CX-BAG-MONUS-NESTED': [(A, B, C) => diff(A, diff(B, C)), (A, B, C) => diff(A, B) | (A & C)],
  };
  for (const [id, [lhs, rhs]] of Object.entries(setForms))
    for (const A of range(U)) for (const B of range(U)) for (const C of range(U)) {
      oneCase(); equal(lhs(A, B, C), rhs(A, B, C), `${id} holds for sets`);
    }
  const bagForms = {
    'CX-BAG-MONUS-SUM-CANCEL': [(a, b) => monus(a + b, b), (a, b) => monus(a, b), '(A sum B) monus B is A, not A monus B'],
    'CX-BAG-MONUS-SUM-DIST': [(a, b, c) => monus(a + b, c), (a, b, c) => monus(a, c) + monus(b, c), 'monus does not distribute over bag sum'],
    'CX-BAG-MONUS-MIN': [(a, b, c) => Math.min(a, monus(b, c)), (a, b, c) => monus(Math.min(a, b), c), 'min with a difference is not the difference of the min'],
    'CX-BAG-MONUS-NESTED': [(a, b, c) => monus(a, monus(b, c)), (a, b, c) => Math.max(monus(a, b), Math.min(a, c)), 'nested difference does not split into difference and min'],
  };
  for (const [id, [lhs, rhs, why]] of Object.entries(bagForms)) {
    const d = firstDifference(N, lhs, rhs);
    check(d !== null, `${id} must have a bag counterexample`);
    counter(id, d.left, d.right, why, { a: d.a, b: d.b, c: d.c });
  }
  // A different weight algebra: the chain 0 < 1 < 2 with max as sum and min as
  // product. Its natural-order monus is b monus a = 0 when b <= a, otherwise b.
  const chainMonus = (b, a) => (b <= a ? 0 : b);
  for (const a of range(3)) for (const b of range(3)) for (const z of range(3)) {
    oneCase(); check((Math.max(a, z) >= b) === (z >= chainMonus(b, a)), 'chain monus is the least z with a+z >= b');
  }
  const a5 = firstDifference(3, (a, b, c) => Math.min(chainMonus(b, a), c), (a, b, c) => chainMonus(Math.min(b, c), Math.min(a, c)));
  check(a5 !== null, 'A5 must fail on the chain');
  counter('CX-MSEMIRING-A5', a5.left, a5.right, 'product does not distribute over monus in the max/min chain on three values', { a: a5.a, b: a5.b, c: a5.c });
});

section('BIMAP-COMPAT', 'all 34 partial bijections on three keys; all pairs and triples', () => {
  const X = range(3);
  const pairsOf = r => r.map((y, x) => [x, y]).filter(([, y]) => y !== null);
  const all = [];
  (function build(i, cur) {
    if (i === X.length) { all.push(pairsOf(cur)); return; }
    build(i + 1, [...cur, null]);
    for (const y of X) if (!cur.includes(y)) build(i + 1, [...cur, y]);
  })(0, []);
  equal(all.length, 34);
  const key = r => JSON.stringify([...r].sort());
  const union = (r, s) => { const m = new Map([...r, ...s].map(p => [String(p), p])); return [...m.values()]; };
  const functionalBothWays = r => new Set(r.map(p => p[0])).size === r.length && new Set(r.map(p => p[1])).size === r.length;
  const inverse = r => r.map(([x, y]) => [y, x]);
  const then = (r, s) => r.flatMap(([x, y]) => s.filter(([y2]) => y2 === y).map(([, z]) => [x, z])); // r first
  const partialIdentity = r => r.every(([x, y]) => x === y);
  const frostCompatible = (r, s) => functionalBothWays(union(r, s));
  const lawsonCompatible = (r, s) => partialIdentity(then(inverse(r), s)) && partialIdentity(then(r, inverse(s)));
  const orthogonal = (r, s) => then(inverse(r), s).length === 0 && then(r, inverse(s)).length === 0;
  for (const r of all) {
    oneCase();
    equal(key(then(r, inverse(r))), key(r.map(([x]) => [x, x])), 'R then inverse is the identity on dom(R)');
    equal(key(then(inverse(r), r)), key(r.map(([, y]) => [y, y])), 'inverse then R is the identity on ran(R)');
    for (const s of all) {
      oneCase();
      equal(frostCompatible(r, s), lawsonCompatible(r, s), 'pair union is a partial bijection iff both mixed products are idempotent');
      if (orthogonal(r, s)) {
        check(frostCompatible(r, s), 'orthogonal implies compatible');
        equal(union(r, s).length, r.length + s.length, 'an orthogonal union is disjoint');
      }
      const disjointEnds = !r.some(([x, y]) => s.some(([x2, y2]) => x === x2 || y === y2));
      equal(orthogonal(r, s), disjointEnds, 'orthogonal means disjoint domains and disjoint ranges');
    }
  }
  let compatiblePairs = 0;
  for (const r of all) for (const s of all) {
    const rs = frostCompatible(r, s);
    if (rs) compatiblePairs++;
    for (const t of all) {
      oneCase();
      equal(functionalBothWays(union(union(r, s), t)), rs && frostCompatible(r, t) && frostCompatible(s, t),
        'a three-way union is a partial bijection iff the operands are pairwise compatible');
      if (rs) {
        const u = union(r, s);
        equal(key(then(u, t)), key(union(then(r, t), then(s, t))), 'composition distributes over a compatible union (union first)');
        equal(key(then(t, u)), key(union(then(t, r), then(t, s))), 'composition distributes over a compatible union (union second)');
        check(functionalBothWays(then(u, t)) && functionalBothWays(then(t, u)), 'the composed result is a partial bijection');
      }
    }
  }
  check(compatiblePairs > 0 && compatiblePairs < all.length * all.length, 'compatibility is neither empty nor total');
  const r = [[0, 0]], s = [], t = [[0, 1]];
  check(frostCompatible(r, s) && frostCompatible(s, t), 'both pairs with the empty bijection are compatible');
  counter('CX-BIMAP-COMPAT-TRANSITIVE', frostCompatible(r, s) && frostCompatible(s, t), frostCompatible(r, t),
    'compatibility is not transitive: {(0,0)} and {(0,1)} are each compatible with the empty bijection', { r, s, t });
});

section('CLOSURE-STABLE', 'elements 0..4, exponents 0..30; three-node weighted graphs, 24 naive iterations; caps 1..5', () => {
  // u^(p) = 1 + u + u^2 + ... + u^p in a semiring given by (plus, times, one).
  const star = (sr, u, p) => { let acc = sr.one, pow = sr.one; for (let i = 1; i <= p; i++) { pow = sr.times(pow, u); acc = sr.plus(acc, pow); } return acc; };
  const stableAt = (sr, u, limit) => range(limit + 1).find(p => star(sr, u, p) === star(sr, u, p + 1));
  const nat = { zero: 0, one: 1, plus: (a, b) => a + b, times: (a, b) => a * b };
  const bool = { zero: 0, one: 1, plus: (a, b) => a | b, times: (a, b) => a & b };
  const tropical = { zero: Infinity, one: 0, plus: Math.min, times: (a, b) => a + b };
  const capped = M => ({ zero: 0, one: 1, plus: (a, b) => Math.min(M, a + b), times: (a, b) => Math.min(M, a * b) });
  equal(stableAt(nat, 0, 30), 0, 'zero is 0-stable in N');
  for (const u of [1, 2, 3, 4]) {
    oneCase(); equal(stableAt(nat, u, 30), undefined, 'a positive natural is not p-stable for any p up to 30');
    for (const p of range(30)) check(star(nat, u, p) < star(nat, u, p + 1), 'the partial sums strictly increase');
  }
  for (const u of [0, 1]) { oneCase(); equal(stableAt(bool, u, 30), 0, 'the Boolean semiring is 0-stable'); }
  for (const u of range(5)) { oneCase(); equal(stableAt(tropical, u, 30), 0, 'min-plus over the naturals is 0-stable'); }
  for (const M of [1, 2, 3, 4, 5]) {
    const sr = capped(M);
    for (const u of range(M + 1)) { oneCase(); check(stableAt(sr, u, 30) <= M, 'a count capped at M is p-stable with p <= M'); }
    for (const a of range(21)) for (const b of range(21)) {
      oneCase();
      equal(Math.min(M, a + b), sr.plus(Math.min(M, a), Math.min(M, b)), 'capping commutes with sum');
      equal(Math.min(M, a * b), sr.times(Math.min(M, a), Math.min(M, b)), 'capping commutes with product');
    }
  }
  // Naive evaluation of T := I + A.T (reflexive-transitive closure) from zero.
  const n = 3;
  const mat = (sr, f) => range(n).map(i => range(n).map(j => f(i, j)));
  const step = (sr, A, T) => mat(sr, (i, j) => sr.plus(i === j ? sr.one : sr.zero, range(n).reduce((acc, k) => sr.plus(acc, sr.times(A[i][k], T[k][j])), sr.zero)));
  const iterate = (sr, A, rounds) => { let T = mat(sr, () => sr.zero); const seen = []; for (let r = 0; r < rounds; r++) { T = step(sr, A, T); seen.push(JSON.stringify(T)); } return seen; };
  const fixedAfter = seen => seen.findIndex((s, i) => i > 0 && s === seen[i - 1]);
  const cycle = (sr) => mat(sr, (i, j) => (j === (i + 1) % n ? sr.one : sr.zero));
  const chain = (sr) => mat(sr, (i, j) => (j === i + 1 ? sr.one : sr.zero));
  const natCycle = iterate(nat, cycle(nat), 24), boolCycle = iterate(bool, cycle(bool), 24), natChain = iterate(nat, chain(nat), 24);
  oneCase(); equal(fixedAfter(natCycle), -1, 'path counts on a cycle do not reach a fixed point in 24 rounds over N');
  oneCase(); check(fixedAfter(boolCycle) > 0 && fixedAfter(boolCycle) <= n + 1, 'Boolean reachability on the same cycle is fixed within n+1 rounds');
  oneCase(); check(fixedAfter(natChain) > 0 && fixedAfter(natChain) <= n + 1, 'path counts on an acyclic chain are fixed within n+1 rounds over N');
  equal(JSON.parse(natCycle[23]).map(row => row.map(v => (v > 0 ? 1 : 0))), JSON.parse(boolCycle[23]), 'the support of the diverging counts is the Boolean closure');
  const total = seen => JSON.parse(seen).flat().reduce((x, y) => x + y, 0);
  for (const r of range(23)) check(total(natCycle[r]) < total(natCycle[r + 1]), 'the total path count grows in every round');
  counter('CX-CLOSURE-N-DIVERGES', total(natCycle[22]), total(natCycle[23]),
    'weighted closure over N keeps growing on a cycle; the same graph over the Booleans is fixed', { graph: 'three-node directed cycle, unit weights', observed: 'sum of all path counts', rounds: [23, 24] });
});

section('KNOW-BOUNDS as three-valued membership', 'one key, all nine pairs of states; two keys, all interval pairs', () => {
  // A [L,U] pair gives each key one of: 2 certain member, 1 possible only, 0 certain non-member.
  const K = { or: Math.max, and: Math.min, not: v => 2 - v };
  const state = (iv, k) => (((iv.L >> k) & 1) ? 2 : ((iv.U >> k) & 1) ? 1 : 0);
  const intervals = [];
  for (const L of range(4)) for (const U of range(4)) if ((L & ~U) === 0) intervals.push({ L, U });
  equal(intervals.length, 9);
  const rules = {
    union: [(a, b) => ({ L: a.L | b.L, U: a.U | b.U }), (x, y) => K.or(x, y)],
    intersection: [(a, b) => ({ L: a.L & b.L, U: a.U & b.U }), (x, y) => K.and(x, y)],
    difference: [(a, b) => ({ L: a.L & ~b.U, U: a.U & ~b.L }), (x, y) => K.and(x, K.not(y))],
    xor: [(a, b) => ({ L: (a.L & ~b.U) | (b.L & ~a.U), U: (a.U & ~b.L) | (b.U & ~a.L) }), (x, y) => K.or(K.and(x, K.not(y)), K.and(y, K.not(x)))],
  };
  for (const [name, [transfer, connective]] of Object.entries(rules))
    for (const a of intervals) for (const b of intervals) {
      const out = transfer(a, b);
      check((out.L & ~out.U) === 0, 'the transfer result is a valid interval');
      for (const k of range(2)) { oneCase(); equal(state(out, k), connective(state(a, k), state(b, k)), `${name} is the strong Kleene connective per key`); }
    }
});

section('CTX-KEY', 'two subjects, three request contexts, all ordered request pairs', () => {
  // One permission: an uncaveated path intersected with a caveated path.
  // Member when the caveat input is true, Conditional when it is missing, NotMember when false.
  const evaluate = ({ subject, caveatInput }) => (subject !== 'alice' ? 'NotMember' : caveatInput === true ? 'Member' : caveatInput === undefined ? 'Conditional' : 'NotMember');
  const contexts = [true, false, undefined];
  const requests = ['alice', 'bob'].flatMap(subject => contexts.map(caveatInput => ({ subject, caveatInput })));
  const cached = keyOf => { const store = new Map(); return q => { const k = keyOf(q); if (!store.has(k)) store.set(k, evaluate(q)); return store.get(k); }; };
  const fullKey = q => JSON.stringify([q.subject, String(q.caveatInput)]);
  const shortKey = q => JSON.stringify([q.subject]);
  let poisoned = 0;
  for (const first of requests) for (const second of requests) {
    oneCase();
    const full = cached(fullKey); full(first);
    equal(full(second), evaluate(second), 'a key that includes the request context returns the direct answer');
    const short = cached(shortKey); short(first);
    if (short(second) !== evaluate(second)) poisoned++;
  }
  check(poisoned > 0, 'a key that omits the request context returns a stale answer for some pair');
  const short = cached(shortKey); short({ subject: 'alice', caveatInput: true });
  counter('CX-CTX-CACHE-KEY', short({ subject: 'alice', caveatInput: undefined }), evaluate({ subject: 'alice', caveatInput: undefined }),
    'a memo key without the caveat input returns Member where the direct answer is Conditional', { first: 'caveat input true', second: 'caveat input missing' });
});

const output = {
  schema: 'frost-prior-art-supplement-checks/v1',
  repository_base: '27b14399b0245df2a018fd5eb6f24f47ce7c2dbe',
  execution: 'independent finite mathematics; no Frost source imported or executed',
  result: 'passed',
  groups,
  total_assertions: groups.reduce((n, x) => n + x.assertions, 0),
  total_cases: groups.reduce((n, x) => n + x.cases, 0),
  computed_counterexamples: counters.length,
  counterexamples: counters,
  limitations: ['finite domains only', 'no Rust compilation', 'no ownership/aliasing proof', 'no solver proof', 'no performance measurement',
    'the stability theorem itself is cited, not proved here; only its instances on the listed bounds are checked',
    'the CTX-KEY group is a three-outcome toy model, not a model of any authorization engine',
    'assertion counts are loop iterations, not a coverage measure; no mutation run covers this file'],
};
console.log(JSON.stringify(output, null, 2));
