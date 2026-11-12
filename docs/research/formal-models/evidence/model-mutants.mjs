/** Mutation sensitivity of model-checks.mjs. Research tooling, not a build or CI step.
 * Run: node docs/research/formal-models/evidence/model-mutants.mjs
 * Each mutant is one textual change to the model source, run in a child Node process fed
 * on standard input. Nothing is written to disk. "killed" means some assertion failed;
 * "survived" means the model accepted the wrong definition. This says which definitions the
 * checks pin. It is not cargo-mutants, and it says nothing about Frost's Rust code.
 */
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const source = readFileSync(fileURLToPath(new URL('./model-checks.mjs', import.meta.url)), 'utf8');
const mutants = [
  ['intersection keeps the right representative', "if(op==='intersection')return x!==null&&y!==null?x:null;", "if(op==='intersection')return x!==null&&y!==null?y:null;"],
  ['union keeps the right representative', "if(op==='union')return x??y;", "if(op==='union')return y??x;"],
  ['sorted union emits a duplicate for a shared key', "else if(a[i]===b[j]){if(op==='union'||op==='intersection')out.push(a[i]);i++;j++;}", "else if(a[i]===b[j]){if(op==='union'||op==='intersection')out.push(a[i]);if(op==='union')out.push(b[j]);i++;j++;}"],
  ['delta replay ignores deletions', "op==='add'?s|(1<<k):s&~(1<<k),base)", "op==='add'?s|(1<<k):s,base)"],
  ['delta replay is an unrelated fold', "es.reduce((s,[op,k])=>op==='add'?s|(1<<k):s&~(1<<k),base)", "es.reduce((s,[op,k])=>(s*7+k+(op==='add'?1:3))%4,base)"],
  ['checked BiMap union never reports a conflict', "a===null||b===null||!bimapValid(a|b,3)?null:a|b", "a===null||b===null?null:a|b"],
  ['BiMap validity checks the left side only', "new Set(ps.map(p=>p[0])).size===ps.length&&new Set(ps.map(p=>p[1])).size===ps.length", "new Set(ps.map(p=>p[0])).size===ps.length"],
  ['bag min is a product', "min=Math.min", "min=(a,b)=>a*b"],
  ['bag monus is the left count', "monus=(a,b)=>Math.max(a-b,0)", "monus=(a,b)=>a"],
  ['checked addition wraps instead of failing', "a===null||b===null||a+b>3?null:a+b", "a===null||b===null?null:(a+b)%4"],
  ['alignment tags a left-only value as right', "{tag:'left',value:x}", "{tag:'right',value:x}"],
  ['weighted composition drops multiplicity', "out[a*n+c]+=r[a*n+b]*s[b*n+c]", "out[a*n+c]=Math.max(out[a*n+c],r[a*n+b]*s[b*n+c])"],
  ['relation composition uses only the first middle key', "for(let b=0;b<n;b++)if((r&(1<<(a*n+b)))&&(s&(1<<(b*n+c))))out|=1<<(a*n+c);return out;", "for(let b=0;b<1;b++)if((r&(1<<(a*n+b)))&&(s&(1<<(b*n+c))))out|=1<<(a*n+c);return out;"],
  ['difference lower bound uses the deny lower bound', "[la&~ub,ua&~lb]", "[la&~lb,ua&~lb]"],
  ['knowledge bounds are sound but not tight', "[la&~ub,ua&~lb],[(la&~ub)|(lb&~ua),(ua&~lb)|(ub&~la)]", "[la&~ub,ua],[0,(ua&~lb)|(ub&~la)]"],
  ['normalizer returns a constant', "const normalize=x=>Number(x.trim());", "const normalize=x=>7;"],
  ['validator accepts 101', "x>=0&&x<=100", "x>=0&&x<=101"],
  ['bag pushforward takes a maximum instead of a sum', "reduce((n,k)=>n+(f[k]===l?b[k]:0),0)", "reduce((n,k)=>Math.max(n,f[k]===l?b[k]:0),0)"],
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
  target: 'docs/research/formal-models/evidence/model-checks.mjs',
  mutants: results.length, killed: count('killed'), survived: count('survived'), not_applied: count('not-applied'),
  results,
  limitations: ['hand-chosen textual mutants of a JavaScript model', 'no Frost Rust code is mutated or executed', 'a killed mutant shows one definition is pinned, not that a law is proved'],
}, null, 2));
if (count('survived') + count('not-applied') > 0) process.exitCode = 1;
