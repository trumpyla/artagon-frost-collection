/** Mutation sensitivity of prior-art-checks.mjs. Research tooling, not a build or CI step.
 * Run: node docs/research/formal-models/evidence/prior-art-mutants.mjs
 * Each mutant is one textual change to the checks source, run in a child Node process fed
 * on standard input. Nothing is written to disk. "killed" means some assertion failed;
 * "survived" means the checks accepted the wrong definition. It is not cargo-mutants, and
 * it says nothing about Frost's Rust code.
 */
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const source = readFileSync(fileURLToPath(new URL('./prior-art-checks.mjs', import.meta.url)), 'utf8');
const mutants = [
  ['monus is absolute difference', 'const monus = (a, b) => Math.max(a - b, 0);', 'const monus = (a, b) => Math.abs(a - b);'],
  ['chain monus is numeric subtraction', 'const chainMonus = (b, a) => (b <= a ? 0 : b);', 'const chainMonus = (b, a) => Math.max(b - a, 0);'],
  ['partial-bijection validity checks the left side only', 'const functionalBothWays = r => new Set(r.map(p => p[0])).size === r.length && new Set(r.map(p => p[1])).size === r.length;', 'const functionalBothWays = r => new Set(r.map(p => p[0])).size === r.length;'],
  ['algebraic compatibility tests one mixed product', 'partialIdentity(then(inverse(r), s)) && partialIdentity(then(r, inverse(s)))', 'partialIdentity(then(inverse(r), s))'],
  ['natural-number sum is bitwise or', 'const nat = { zero: 0, one: 1, plus: (a, b) => a + b,', 'const nat = { zero: 0, one: 1, plus: (a, b) => a | b,'],
  ['capped product is not capped', 'times: (a, b) => Math.min(M, a * b) });', 'times: (a, b) => a * b });'],
  ['difference lower bound uses the deny lower bound', 'difference: [(a, b) => ({ L: a.L & ~b.U,', 'difference: [(a, b) => ({ L: a.L & ~b.L,'],
  ['full memo key drops the request context', 'const fullKey = q => JSON.stringify([q.subject, String(q.caveatInput)]);', 'const fullKey = q => JSON.stringify([q.subject]);'],
];
const results = [];
for (const [name, from, to] of mutants) {
  if (!source.includes(from)) { results.push({ name, outcome: 'not-applied' }); continue; }
  const run = spawnSync(process.execPath, ['--input-type=module', '-'], { input: source.replace(from, to), encoding: 'utf8', maxBuffer: 1 << 26 });
  const failed = (run.stderr.match(/AssertionError[^\n]*/) || [''])[0].slice(0, 120);
  results.push(run.status === 0 ? { name, outcome: 'survived' } : { name, outcome: 'killed', first_failure: failed });
}
const count = outcome => results.filter(r => r.outcome === outcome).length;
console.log(JSON.stringify({
  schema: 'frost-formal-model-mutants/v1',
  target: 'docs/research/formal-models/evidence/prior-art-checks.mjs',
  mutants: results.length, killed: count('killed'), survived: count('survived'), not_applied: count('not-applied'),
  results,
  limitations: ['hand-chosen textual mutants of a JavaScript model', 'no Frost Rust code is mutated or executed', 'a killed mutant shows one definition is pinned, not that a law is proved'],
}, null, 2));
if (count('survived') + count('not-applied') > 0) process.exitCode = 1;
