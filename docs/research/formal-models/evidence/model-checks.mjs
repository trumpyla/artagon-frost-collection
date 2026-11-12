/** Independent finite mathematics, not Frost execution, Rust verification or an unbounded proof.
 * Run: node docs/research/formal-models/evidence/model-checks.mjs
 * No dependencies, network, wall-clock correctness assertions, or repository writes.
 * A "computed" counterexample evaluates model functions on both sides. An "illustration"
 * only records two hand-written values that differ; it is not evidence about any model.
 * model-mutants.mjs applies real mutations to this file and reports which checks catch them.
 */
import assert from 'node:assert/strict';
const groups = new Map();
const counters = [];
let group;
function section(name, bounds, fn) {
  group = { name, bounds, assertions: 0, cases: 0 };
  groups.set(name, group);
  fn();
}
function check(value, message) { group.assertions++; assert.ok(value, message); }
function equal(a, b, message) { group.assertions++; assert.deepEqual(a,b,message); }
function oneCase() { group.cases++; }
function counter(id, left, right, why) {
  group.assertions++; assert.notDeepEqual(left,right,id);
  counters.push({id,kind:'computed',left,right,why});
}
function illustrate(id, left, right, why) {
  assert.notDeepEqual(left,right,id);
  counters.push({id,kind:'illustration',left,right,why});
}
const range = n => Array.from({length:n},(_,i)=>i);
const setValues = (s,n=4) => range(n).filter(x=>(s&(1<<x))!==0);
const bitset = xs => xs.reduce((a,x)=>a|(1<<x),0);
const subset = (a,b) => (a & ~b)===0;
const pc = s => setValues(s,16).length;
function words(alphabet,maxLength) { let all=[[]], level=[[]]; for(let i=0;i<maxLength;i++){level=level.flatMap(s=>alphabet.map(v=>[...s,v]));all.push(...level);}return all; }
function partialMaps(keys,values) { return words([null,...values],keys).filter(v=>v.length===keys); }
function sortedKernel(a,b,op) {
  let i=0,j=0,out=[];
  while(i<a.length || j<b.length) {
    if(i===a.length){if(op==='union'||op==='xor')out.push(b[j]);j++;}
    else if(j===b.length){if(op!=='intersection')out.push(a[i]);i++;}
    else if(a[i]===b[j]){if(op==='union'||op==='intersection')out.push(a[i]);i++;j++;}
    else if(a[i]<b[j]){if(op!=='intersection')out.push(a[i]);i++;}
    else {if(op==='union'||op==='xor')out.push(b[j]);j++;}
  }
  return out;
}
section('SET-BOOLEAN and sorted-kernel reference', 'all subsets of four keys; all pairs and triples',()=>{
  for(const a of range(16)) for(const b of range(16)) {
    oneCase();
    for(const [op,s] of [['union',a|b],['intersection',a&b],['difference',a&~b],['xor',a^b]])
      equal(sortedKernel(setValues(a),setValues(b),op),setValues(s),op);
    const i=pc(a&b);
    equal(pc(a|b),pc(a)+(pc(b)-i)); equal(pc(a^b),(pc(a)-i)+(pc(b)-i)); equal(pc(a&~b),pc(a)-i);
    for(const c of range(16)) {
      oneCase();
      equal((a|b)|c,a|(b|c));equal((a&b)&c,a&(b&c));equal((a^b)^c,a^(b^c));
      equal(a&(b|c),(a&b)|(a&c));equal(a|(b&c),(a|b)&(a|c));
      equal(a|(a&b),a);equal(a&(a|b),a);
    }
  }
});
const enrich = (a,b,op) => a.map((x,i)=>{
  const y=b[i];
  if(op==='union')return x??y;
  if(op==='intersection')return x!==null&&y!==null?x:null;
  if(op==='difference')return x!==null&&y===null?x:null;
  return (x===null)!==(y===null)?(x??y):null;
});
section('SET-REP', 'three keys; each absent or representative a/b; all triples',()=>{
  const reps=partialMaps(3,['a','b']);
  for(const a of reps)for(const b of reps)for(const c of reps){
    oneCase();
    equal(enrich(enrich(a,b,'union'),c,'union'),enrich(a,enrich(b,c,'union'),'union'));
    equal(enrich(enrich(a,b,'intersection'),c,'intersection'),enrich(a,enrich(b,c,'intersection'),'intersection'));
    // Orientations that keep the left operand first also hold for representatives.
    equal(enrich(a,enrich(a,b,'intersection'),'union'),a);
    equal(enrich(a,enrich(b,c,'union'),'intersection'),enrich(enrich(a,b,'intersection'),enrich(a,c,'intersection'),'union'));
    equal(enrich(a,enrich(b,c,'intersection'),'union'),enrich(enrich(a,b,'union'),enrich(a,c,'union'),'intersection'));
  }
  // Pin which side supplies the representative; associativity alone holds for either bias.
  equal(enrich(['a'],['b'],'union'),['a']); equal(enrich([null],['b'],'union'),['b']);
  equal(enrich(['a'],['b'],'intersection'),['a']); equal(enrich(['a'],[null],'difference'),['a']);
  equal(enrich(['a'],[null],'xor'),['a']); equal(enrich([null],['b'],'xor'),['b']); equal(enrich(['a'],['b'],'xor'),[null]);
  counter('CX-REP-ABSORPTION',enrich(enrich(['b'],['a'],'intersection'),['a'],'union'),['a'],'(B intersection A) union A returns B\'s representative, not A\'s');
  counter('CX-REP-DISTRIBUTE',enrich(enrich(['b'],[null],'intersection'),['a'],'union'),enrich(enrich(['b'],['a'],'union'),enrich([null],['a'],'union'),'intersection'),'(B intersection C) union A and (B union A) intersection (C union A) select different representatives');
  counter('CX-SET-XOR-REP',enrich(enrich(['a'],['b'],'xor'),['c'],'xor'),enrich(['a'],enrich(['b'],['c'],'xor'),'xor'),'membership XOR association loses the selected representative');
  counter('CX-SET-UNION-COMMUTE',enrich(['a'],['b'],'union'),enrich(['b'],['a'],'union'),'left representatives are not commutative');
  counter('CX-REP-FILTER',enrich(['a'],['b'],'union').filter(x=>x==='b'),enrich(['a'].map(x=>x==='b'?x:null),['b'],'union').filter(x=>x!==null),'representative-sensitive filtering is not key-class filtering');
});
section('DOM-IMAGE, preimage and canonicalization', 'all 27 functions from three keys to three keys; all finite subset pairs',()=>{
 const funcs=words(range(3),3).filter(x=>x.length===3);
 for(const f of funcs){
   const image=s=>bitset(setValues(s,3).map(x=>f[x]));
   const preimage=s=>bitset(range(3).filter(x=>(s&(1<<f[x]))!==0));
   const injective=new Set(f).size===3;
   let allIntersection=true,allDifference=true,allXor=true;
   for(const a of range(8))for(const b of range(8)){
     oneCase(); equal(image(a|b),image(a)|image(b));
     if(image(a&b)!==(image(a)&image(b)))allIntersection=false;
     if(image(a&~b)!==(image(a)&~image(b)))allDifference=false;
     if(image(a^b)!==(image(a)^image(b)))allXor=false;
     // Direction of the error for a collapsing conversion.
     check(subset(image(a&b),image(a)&image(b))); check(subset(image(a)&~image(b),image(a&~b))); check(subset(image(a)^image(b),image(a^b)));
     if(injective){equal(image(a&b),image(a)&image(b));equal(image(a&~b),image(a)&~image(b));equal(image(a^b),image(a)^image(b));}
     equal(preimage(a|b),preimage(a)|preimage(b));equal(preimage(a&b),preimage(a)&preimage(b));equal(preimage(a&~b),preimage(a)&~preimage(b));
   }
   equal(allIntersection,injective); equal(allDifference,injective); equal(allXor,injective);
   const push=b=>range(3).map(l=>range(3).reduce((n,k)=>n+(f[k]===l?b[k]:0),0));
   const small=words(range(3),3).filter(x=>x.length===3);
   for(const a of small)for(const b of small){oneCase();equal(push(a.map((x,i)=>x+b[i])),push(a).map((x,i)=>x+push(b)[i]));}
 }
 const f=x=>0, A=1,B=2,image=s=>bitset(setValues(s,3).map(f));
 counter('CX-COLLAPSED-INTERSECTION',image(A&B),image(A)&image(B),'a and b become the same target key');
 counter('CX-COLLAPSED-DIFFERENCE',image(A&~B),image(A)&~image(B),'projection changes exclusion');
 const constant=x=>0; check(range(3).every(x=>constant(constant(x))===constant(x)),'the constant normalizer is idempotent');
 counter('CX-CANONICAL-IDEMPOTENCE', new Set(range(3).map(constant)).size, 3, 'a constant normalizer is idempotent but not identity-preserving');
});
const bagOp=(a,b,f)=>a.map((x,i)=>f(x,b[i]));
const support=b=>bitset(b.flatMap((x,i)=>x>0?[i]:[]));
const sum=(a,b)=>a+b, max=Math.max,min=Math.min, monus=(a,b)=>Math.max(a-b,0), abs=(a,b)=>Math.abs(a-b);
section('BAG-LAWS, BAG-SUPPORT and count partiality', 'two-key bags with counts 0..3; all triples; checked scalar width 0..3',()=>{
 const bags=words(range(4),2).filter(x=>x.length===2);
 for(const a of bags)for(const b of bags){
   oneCase();equal(support(bagOp(a,b,sum)),support(a)|support(b));
   equal(support(bagOp(a,b,max)),support(a)|support(b));equal(support(bagOp(a,b,min)),support(a)&support(b));
   for(const f of [sum,max,min,abs])equal(bagOp(a,b,f),bagOp(b,a,f));
   for(const c of bags){oneCase();for(const f of [sum,max,min])equal(bagOp(bagOp(a,b,f),c,f),bagOp(a,bagOp(b,c,f),f));
     equal(bagOp(a,bagOp(b,c,min),max),bagOp(bagOp(a,b,max),bagOp(a,c,max),min));
     equal(bagOp(a,bagOp(b,c,max),min),bagOp(bagOp(a,b,min),bagOp(a,c,min),max));}
 }
 // Pin the scalar operations against each other: a = min + monus, max = a + monus(b,a), abs = both residues.
 for(const a of range(4))for(const b of range(4)){oneCase();
   equal(min(a,b)+monus(a,b),a); equal(max(a,b),a+monus(b,a)); equal(abs(a,b),monus(a,b)+monus(b,a));
   check(min(a,b)===a||min(a,b)===b); check(min(a,b)<=a&&min(a,b)<=b); equal(abs(a,a),0);}
 const checked=(a,b)=>a===null||b===null||a+b>3?null:a+b;
 for(const a of range(4))for(const b of range(4))for(const c of range(4)){oneCase();
   equal(checked(checked(a,b),c),checked(a,checked(b,c)));
   equal(checked(checked(a,b),c),a+b+c<=3?a+b+c:null);}
 counter('CX-BAG-ABS-ASSOC',abs(abs(1,1),2),abs(1,abs(1,2)),'absolute difference is not XOR');
 counter('CX-BAG-SUPPORT-MONUS',support([monus(2,1)]),support([2])&~support([1]),'positive residual multiplicity is lost by set subtraction');
 const collapse=b=>[b[0]+b[1]], left=[1,0], right=[0,1];
 counter('CX-BAG-PUSHFORWARD-MAX',collapse(bagOp(left,right,max)),bagOp(collapse(left),collapse(right),max),'coalescing keys changes maximum counts');
 counter('CX-BAG-PUSHFORWARD-MIN',collapse(bagOp(left,right,min)),bagOp(collapse(left),collapse(right),min),'coalescing keys invents overlap');
 counter('CX-BAG-PUSHFORWARD-MONUS',collapse(bagOp(left,right,monus)),bagOp(collapse(left),collapse(right),monus),'coalescing keys cancels a residue that the separate keys keep');
 counter('CX-BAG-SUPPORT-ABS',support([abs(1,2)]),support([1])^support([2]),'absolute difference of unequal positive counts is positive; XOR of equal supports is empty');
 equal(checked(3,1),null); // a wider final answer does not permit intermediate wrapping
});
const merge=(a,b,f)=>a.map((x,i)=>x===null?b[i]:b[i]===null?x:f(x,b[i]));
section('MAP-LIFT and join presence', 'two keys, absence or values 0..2; all triples',()=>{
 const maps=partialMaps(2,[0,1,2]);
 for(const a of maps)for(const b of maps)for(const c of maps){
   oneCase();for(const f of [(x,y)=>x,(x,y)=>y,(x,y)=>x+y]) equal(merge(merge(a,b,f),c,f),merge(a,merge(b,c,f),f));
 }
 const del=(x,y)=>x===y?null:x;
 counter('CX-MAP-DELETE-ASSOC',merge(merge([1],[1],del),[2],del),merge([1],merge([1],[2],del),del),'a deleted binding can reappear differently after reassociation');
 counter('CX-MAP-SUBTRACT-ASSOC',(10-3)-2,10-(3-2),'closure does not imply associativity');
 illustrate('CX-MAP-PRESENT-NONE',{present:true,value:null},{present:false},'stored None is not missing');
 illustrate('CX-MAP-REP-PAYLOAD', {keyTenant:'left',valueTenant:'right'},{keyTenant:'right',valueTenant:'right'},'left key metadata plus right payload may violate a dependent invariant');
});
section('MAP-JOIN and tagged alignment', 'all maps on two keys with absence or values 0..2; all pairs/triples',()=>{
 const maps=partialMaps(2,[0,1,2]);
 const join=(a,b)=>a.map((x,i)=>x!==null&&b[i]!==null?[x,b[i]]:null);
 const align=(a,b)=>a.map((x,i)=>x===null?(b[i]===null?null:{tag:'right',value:b[i]}):b[i]===null?{tag:'left',value:x}:{tag:'both',value:[x,b[i]]});
 const dom=a=>bitset(a.flatMap((v,i)=>v!==null?[i]:[]));
 for(const a of maps)for(const b of maps){
  oneCase(); const j=join(a,b), z=align(a,b);
  equal(dom(j),dom(a)&dom(b)); equal(dom(z),dom(a)|dom(b));
  equal(j,join(b,a).map(v=>v===null?null:[v[1],v[0]]));
  for(const c of maps){
   oneCase();const l=join(join(a,b),c).map(v=>v===null?null:[v[0][0],v[0][1],v[1]]);
   const r=join(a,join(b,c)).map(v=>v===null?null:[v[0],v[1][0],v[1][1]]);
   equal(l,r);
  }
 }
 equal(align([1,null,3],[null,2,4]),[{tag:'left',value:1},{tag:'right',value:2},{tag:'both',value:[3,4]}]);
 equal(align([null],[null]),[null]);
 counter('CX-JOIN-TUPLE-TYPE',join(join([1],[2]),[3]),join([1],join([2],[3])),'join reassociation requires an explicit product isomorphism');
});
section('DOM-VALID, DOM-EQ and subcarrier closure', 'all functions/filters on three keys; percentage boundaries and explicit object metadata',()=>{
 // Separate raw/output types: meaning preservation is not raw object equality.
 const normalize=x=>Number(x.trim());
 // Expected meanings are written out, not recomputed with the normalizer's own expression.
 for(const [raw,meaning] of [[' 7 ',7],['7',7],['\t42\n',42],['0',0]]){oneCase();equal(normalize(raw),meaning);equal(normalize(String(normalize(raw))),meaning);}
 const validate=x=>Number.isInteger(x)&&x>=0&&x<=100;
 for(const [x,valid] of [[-1,false],[0,true],[1,true],[99,true],[100,true],[101,false],[0.5,false],[NaN,false],[80+50,false]]){oneCase();equal(validate(x),valid);}
 const funcs=words(range(3),3).filter(x=>x.length===3);
 for(const f of funcs)for(const bits of range(8)){
  oneCase();const eq=(x,y)=>f[x]===f[y],pred=x=>!!(bits&(1<<x));
  // eq is an equivalence by construction (equal classifier images); DOM-EQ itself is an obligation, not a checked law.
  const congruent=range(3).every(x=>range(3).every(y=>!eq(x,y)||pred(x)===pred(y)));
  const representativeAnswers=new Map();let constantOnClasses=true;
  for(const x of range(3)){if(representativeAnswers.has(f[x])&&representativeAnswers.get(f[x])!==pred(x))constantOnClasses=false;representativeAnswers.set(f[x],pred(x));}
  equal(congruent,constantOnClasses);
 }
 counter('CX-NONEMPTY-CLOSURE',[pc(1)>0,pc(2)>0,pc(1&2)>0],[true,true,true],'nonempty inputs can have empty intersection');
 counter('CX-CARDINALITY-CAP',pc(1|2)<=1,true,'a one-element collection bound is not closed under union');
 const key=x=>x.id, a={id:7,label:'a'}, b={id:7,label:'b'};
 equal(key(a),key(b)); illustrate('CX-DERIVED-IDENTITY',JSON.stringify(a),JSON.stringify(b),'whole-record structural equality differs from selected-key identity');
 check(!(NaN===NaN),'ordinary NaN equality is not reflexive');
});
section('SEQ-MONOID and occurrence projection', 'alphabet a/b, all words of length at most three, all triples',()=>{
 const ws=words(['a','b'],3);const cat=(a,b)=>[...a,...b],rev=a=>[...a].reverse();
 const count=a=>['a','b'].map(x=>a.filter(y=>y===x).length);
 for(const a of ws)for(const b of ws){
   oneCase();equal(rev(cat(a,b)),cat(rev(b),rev(a)));
   equal(count(cat(a,b)),bagOp(count(a),count(b),sum));
   equal(cat(a,b).filter(x=>x==='a'),cat(a.filter(x=>x==='a'),b.filter(x=>x==='a')));
   for(const c of ws){oneCase();equal(cat(cat(a,b),c),cat(a,cat(b,c)));}
 }
 const zip=(a,b)=>a.slice(0,b.length).map((x,i)=>[x,b[i]]);
 counter('CX-ZIP-CONCAT',zip(['a','b'],['c']),[...zip(['a'],[]),...zip(['b'],['c'])],'length-misaligned concat changes pair positions');
 const bagMin=(s,t)=>{const left=[...t];return s.filter(x=>{const i=left.indexOf(x);if(i<0)return false;left.splice(i,1);return true;});};
 counter('CX-SEQ-INTERSECTION',['a','a'].filter(x=>['a'].includes(x)),bagMin(['a','a'],['a']),'membership-filter and bag-min intersection differ');
});
function relationCompose(r,s,n=2){let out=0;for(let a=0;a<n;a++)for(let c=0;c<n;c++)for(let b=0;b<n;b++)if((r&(1<<(a*n+b)))&&(s&(1<<(b*n+c))))out|=1<<(a*n+c);return out;}
function weightedCompose(r,s,n=2){const out=Array(n*n).fill(0);for(let a=0;a<n;a++)for(let c=0;c<n;c++)for(let b=0;b<n;b++)out[a*n+c]+=r[a*n+b]*s[b*n+c];return out;}
section('REL-COMP and BAGREL-COMP', 'all Boolean relations on two-by-two keys; all triples; natural weighted products',()=>{
 for(const r of range(16))for(const s of range(16))for(const t of range(16)){
   oneCase();equal(relationCompose(relationCompose(r,s),t),relationCompose(r,relationCompose(s,t)));
   equal(relationCompose(r,s|t),relationCompose(r,s)|relationCompose(r,t));
   const weight=x=>range(4).map(i=>(x>>i)&1);
   equal(weightedCompose(weightedCompose(weight(r),weight(s)),weight(t)),weightedCompose(weight(r),weightedCompose(weight(s),weight(t))));
 }
 // Two separately written compositions must agree on support; weights range over 0..2, not just 0/1.
 const weights=words(range(3),4).filter(x=>x.length===4), supp=w=>bitset(w.flatMap((x,i)=>x>0?[i]:[]));
 for(const r of weights)for(const s of weights){oneCase();equal(supp(weightedCompose(r,s)),relationCompose(supp(r),supp(s)));}
 for(const r of weights.filter((_,i)=>i%3===0))for(const s of weights.filter((_,i)=>i%2===0))for(const t of weights.filter((_,i)=>i%5===0)){
   oneCase();equal(weightedCompose(weightedCompose(r,s),t),weightedCompose(r,weightedCompose(s,t)));}
 counter('CX-REL-WITNESSES',weightedCompose([1,1,0,0],[1,0,1,0])[0],pc(relationCompose(3,5)),'two derivations become one Boolean pair');
 const normalizeFibers=m=>Object.fromEntries(Object.entries(m).filter(([,fiber])=>fiber.length>0));
 counter('CX-MM-PRESENT-EMPTY',{k:[]},normalizeFibers({k:[]}),'native map empty fiber is not automatically normalized absence');
 illustrate('CX-MM-GROUP-REP',['left-key','right-only-value'],['right-key','right-only-value'],'group key and pair-origin contracts differ');
 const entries=[['a',1],['b',2],['a',3]], grouped=new Map();
 for(const [k,v] of entries){if(!grouped.has(k))grouped.set(k,[]);grouped.get(k).push(v);}
 counter('CX-MM-FLATTEN',[...grouped].flatMap(([k,vs])=>vs.map(v=>[k,v])),entries,'grouping loses global interleaving');
});
const pairs=(r,n)=>range(n*n).filter(i=>r&(1<<i)).map(i=>[Math.floor(i/n),i%n]);
const bimapValid=(r,n)=>{const ps=pairs(r,n);return new Set(ps.map(p=>p[0])).size===ps.length&&new Set(ps.map(p=>p[1])).size===ps.length;};
const inverse=(r,n)=>bitset(pairs(r,n).map(([a,b])=>b*n+a));
section('BIMAP-CLOSED, BIMAP-IDENTITY, BIMAP-PARTIAL', 'all 34 partial bijections over three keys; all pairs/triples',()=>{
 const bs=range(512).filter(x=>bimapValid(x,3));equal(bs.length,34);
 const checkedUnion=(a,b)=>a===null||b===null||!bimapValid(a|b,3)?null:a|b;
 for(const r of bs){
   equal(inverse(inverse(r,3),3),r);
   const domainId=bitset(pairs(r,3).map(([a])=>a*3+a));
   const rangeId=bitset(pairs(r,3).map(([,b])=>b*3+b));
   equal(relationCompose(r,inverse(r,3),3),domainId);equal(relationCompose(inverse(r,3),r,3),rangeId);
   for(const s of bs){
     oneCase();check(bimapValid(r&s,3));check(bimapValid(r&~s,3));check(bimapValid(relationCompose(r,s,3),3));
     equal(inverse(relationCompose(r,s,3),3),relationCompose(inverse(s,3),inverse(r,3),3));
     for(const t of bs){
       oneCase();equal(relationCompose(relationCompose(r,s,3),t,3),relationCompose(r,relationCompose(s,t,3),3));
       equal(checkedUnion(checkedUnion(r,s),t),checkedUnion(r,checkedUnion(s,t)));
       equal(checkedUnion(checkedUnion(r,s),t),bimapValid(r|s|t,3)?r|s|t:null);
     }
   }
 }
 const xor=(a,b)=>a===null||b===null||!bimapValid(a^b,2)?null:a^b;
 counter('CX-BIMAP-XOR-DEFINEDNESS',xor(xor(1,2),2),xor(1,xor(2,2)),'intermediate conflict changes success/failure');
 counter('CX-BIMAP-UNION-CLOSURE',bimapValid(1|2,2),true,'two partners for one left key');
 counter('CX-BIMAP-RIGHT-UNIQUENESS',bimapValid(1|4,2),true,'two left keys for one right key');
 const keyDifference=(r,s,n)=>bitset(pairs(r,n).filter(([a])=>!pairs(s,n).some(([x])=>x===a)).map(([a,b])=>a*n+b));
 counter('CX-BIMAP-PAIR-KEY-DIFF',1&~2,keyDifference(1,2,2),'equal left key but different pair');
});
section('KNOW-BOUNDS', 'all sound lower/upper intervals and all compatible concrete realizations over three keys',()=>{
 const intervals=[];for(const l of range(8))for(const u of range(8))if(subset(l,u))intervals.push([l,u]);
 equal(intervals.length,27);
 for(const [la,ua] of intervals)for(const [lb,ub] of intervals){
  const meet=[7,7,7,7], join=[0,0,0,0];
  const rules=[[la|lb,ua|ub],[la&lb,ua&ub],[la&~ub,ua&~lb],[(la&~ub)|(lb&~ua),(ua&~lb)|(ub&~la)]];
  for(const a of range(8).filter(a=>subset(la,a)&&subset(a,ua)))for(const b of range(8).filter(b=>subset(lb,b)&&subset(b,ub))){
   oneCase();
   [a|b,a&b,a&~b,a^b].forEach((actual,i)=>{
    check(subset(rules[i][0],actual));check(subset(actual,rules[i][1]));
    meet[i]&=actual;join[i]|=actual;
   });
  }
  // For independent operands each rule is exact: the bounds are the meet and join of all realizations.
  rules.forEach(([lo,hi],i)=>{equal(lo,meet[i]);equal(hi,join[i]);});
 }
 counter('CX-INCOMPLETE-DENY',1&~0,1&~1,'using a deny lower bound as complete absence invents an allowed member');
});
section('DELTA-REPLAY and fixed points', 'two keys, all insert/delete histories up to length four and cuts; all three-node graphs',()=>{
 const events=[['add',0],['add',1],['del',0],['del',1]];
 const replay=(base,es)=>es.reduce((s,[op,k])=>op==='add'?s|(1<<k):s&~(1<<k),base);
 const lastWins=(base,es)=>bitset(range(2).filter(k=>{const last=es.filter(([,x])=>x===k).pop();return last?last[0]==='add':(base&(1<<k))!==0;}));
 for(const base of range(4))for(const es of words(events,4)){
   oneCase();equal(replay(base,es),lastWins(base,es));
   for(const cut of range(es.length+1)){oneCase();equal(replay(replay(base,es.slice(0,cut)),es.slice(cut)),replay(base,es));}
 }
 counter('CX-DELTA-ORDER',replay(0,[['del',0],['add',0]]),(0|1)&~1,'union of all inserts minus all deletes ignores last-write order');
 for(const graph of range(512))for(const start of range(3)){
   oneCase();let reached=1<<start,steps=0;
   for(;;){let next=reached;for(const [a,b]of pairs(graph,3))if(reached&(1<<a))next|=1<<b;if(next===reached)break;reached=next;steps++;check(steps<=3);}
   const seen=new Set([start]),todo=[start];while(todo.length){const a=todo.shift();for(const [x,y]of pairs(graph,3))if(x===a&&!seen.has(y)){seen.add(y);todo.push(y);}}
   equal(reached,bitset([...seen]));
 }
});
section('Domain contexts and frozen observation', 'explicit hostile descriptors, identity metadata, and abstract heap frames',()=>{
 const compatible=(a,b)=>a.kind===b.kind&&a.tenant===b.tenant&&a.dictionary===b.dictionary&&a.revision===b.revision;
 const base={kind:'principal',tenant:'A',dictionary:'v1',revision:1};
 const checkedUnion=(a,b)=>compatible(a.context,b.context)?{complete:a.members|b.members}:'IncompatibleContext';
 const shortcutUnion=(a,b)=>a.members===0?{complete:b.members}:b.members===0?{complete:a.members}:checkedUnion(a,b);
 equal(checkedUnion({context:base,members:1},{context:{...base},members:2}),{complete:3});
 for(const [field,v]of [['kind','resource'],['tenant','B'],['dictionary','v2'],['revision',2]]){
   oneCase();const other={context:{...base,[field]:v},members:0};
   equal(checkedUnion({context:base,members:1},other),'IncompatibleContext');
   equal(shortcutUnion({context:base,members:1},other),{complete:1});
 }
 const otherTenant={context:{...base,tenant:'B'},members:0};
 counter('CX-EMPTY-DOMAIN-SHORTCUT',shortcutUnion({context:base,members:1},otherTenant),checkedUnion({context:base,members:1},otherTenant),'an empty-operand shortcut hides a required compatibility error');
 const members=[1],heap=new Map([[1,'before']]),denote=()=>members.map(k=>heap.get(k));
 const before=denote();heap.set(1,'after');counter('CX-FROZEN-DEEP-STABILITY',before,denote(),'fixed membership does not freeze external payload state');
 illustrate('CX-OWNED-VS-BORROWED',{containerOwned:true,sourceLifetimeRequired:true},{containerOwned:true,sourceLifetimeRequired:false},'materialized references are not lifetime-independent');
});
const output={
 schema:'frost-independent-formal-model-results/v1',
 repository_base:'27b14399b0245df2a018fd5eb6f24f47ce7c2dbe',
 execution:'independent finite mathematics; no Frost source imported or executed',
 result:'passed',
 groups:[...groups.values()],
 total_assertions:[...groups.values()].reduce((n,x)=>n+x.assertions,0),
 total_cases:[...groups.values()].reduce((n,x)=>n+x.cases,0),
 computed_counterexamples:counters.filter(c=>c.kind==='computed').length,
 illustrations:counters.filter(c=>c.kind==='illustration').length,
 counterexamples:counters,
 limitations:['finite domains only','no Rust compilation','no ownership/aliasing proof','no solver proof','no performance measurement','assertion counts are loop iterations, many over native bit operators; they are not a coverage measure','illustrations are differing literals, not model results','mutation sensitivity is reported separately by model-mutants.mjs']
};
console.log(JSON.stringify(output,null,2));
