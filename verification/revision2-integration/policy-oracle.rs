use frost::integer::{FrozenIntSet, IntSetBackend, IntSetError, IntSetPolicy, IntSetRepr};
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

#[derive(Default, Debug, Eq, PartialEq)]
struct Events(Vec<(u8,Vec<u8>)>);
impl Hasher for Events {
    fn finish(&self)->u64 {0}
    fn write(&mut self,b:&[u8]) {self.0.push((0,b.to_vec()));}
    fn write_u8(&mut self,v:u8) {self.0.push((8,v.to_ne_bytes().to_vec()));}
    fn write_u64(&mut self,v:u64) {self.0.push((64,v.to_ne_bytes().to_vec()));}
    fn write_usize(&mut self,v:usize) {self.0.push((1,v.to_ne_bytes().to_vec()));}
}
fn events(v:&FrozenIntSet<u64>)->Events {let mut e=Events::default();v.hash(&mut e);e}
fn make(v:&[u64],p:IntSetPolicy)->Option<FrozenIntSet<u64>> {
    match FrozenIntSet::try_from_iter(v.iter().copied(),p) {
        Ok(v)=>Some(v),
        Err(IntSetError::DenseBudgetExceeded{..}|IntSetError::RoaringPartitionLimit{..})=>None,
        Err(e)=>panic!("unexpected input error: {e}"),
    }
}
fn expected(a:&[u64],b:&[u64],op:usize)->Vec<u64> {
    let a:BTreeSet<_>=a.iter().copied().collect(); let b:BTreeSet<_>=b.iter().copied().collect();
    match op {0=>a.union(&b).copied().collect(),1=>a.intersection(&b).copied().collect(),2=>a.difference(&b).copied().collect(),_=>a.symmetric_difference(&b).copied().collect()}
}
fn result(a:&FrozenIntSet<u64>,b:&FrozenIntSet<u64>,op:usize,p:IntSetPolicy)->Result<FrozenIntSet<u64>,IntSetError> {
    match op {0=>a.union_with_policy(b,p),1=>a.intersection_with_policy(b,p),2=>a.difference_with_policy(b,p),_=>a.symmetric_difference_with_policy(b,p)}
}
fn main() {
    let mut shapes=vec![vec![],vec![0],vec![1],vec![63,64],vec![0,u64::MAX],vec![u64::MAX-129,u64::MAX-64,u64::MAX],vec![(1u64<<32)-65,(1u64<<32)+64],vec![0,(1u64<<32),(1u64<<48)]];
    shapes.push((0..130).collect());
    shapes.push((u64::MAX-129..=u64::MAX).collect());
    shapes.push((0..20).flat_map(|i|(0..5).map(move|j|65530+i*65536+j)).collect());
    shapes.push((0..100).map(|i|i*7).collect());
    let backends=[IntSetPolicy::sorted(),IntSetPolicy::dense(),IntSetPolicy::roaring(),IntSetPolicy::chunked()];
    let mut checks=0usize;
    for a in &shapes {for b in &shapes {for pa in backends {for pb in backends {
        let (Some(a_set),Some(b_set))=(make(a,pa),make(b,pb)) else {continue};
        for op in 0..4 {
            let answer=expected(a,b,op);
            let sorted=make(&answer,IntSetPolicy::sorted()).unwrap();
            let expected_events=events(&sorted);
            let dense_bytes=answer.first().zip(answer.last()).map_or(0,|(&lo,&hi)|((u128::from(hi)-u128::from(lo&!63))/64+1)*8);
            let partitions=answer.iter().map(|x|x>>32).collect::<BTreeSet<_>>().len();
            for base in backends.into_iter().chain([IntSetPolicy::auto()]) {for dense in [0,8,16,1024] {for cap in [0,1,2] {
                let p=base.with_dense_budget(dense).with_roaring_partition_limit(cap);
                let r=result(&a_set,&b_set,op,p);
                if p.backend()==IntSetBackend::Dense && dense_bytes>dense as u128 {
                    assert!(matches!(r,Err(IntSetError::DenseBudgetExceeded{required_bytes,max_bytes}) if required_bytes==dense_bytes && max_bytes==dense));
                } else if p.backend()==IntSetBackend::Roaring && partitions>cap {
                    assert!(matches!(r,Err(IntSetError::RoaringPartitionLimit{partitions:p,max_partitions}) if p==partitions as u64 && max_partitions==cap));
                } else {
                    let r=r.unwrap();
                    assert_eq!(r.iter().collect::<Vec<_>>(),answer);
                    assert_eq!(r,sorted);
                    assert_eq!(events(&r),expected_events);
                    assert_eq!(r.limits(),p.limits());
                    match r.representation() {
                        IntSetRepr::Dense=>assert!(r.contiguous_payload_bytes().unwrap()<=dense),
                        IntSetRepr::Roaring=>assert!(partitions<=cap),
                        _=>{},
                    }
                    if p.backend()!=IntSetBackend::Auto {
                        let repr=match p.backend() {IntSetBackend::Sorted=>IntSetRepr::Sorted,IntSetBackend::Dense=>IntSetRepr::Dense,IntSetBackend::Roaring=>IntSetRepr::Roaring,IntSetBackend::Chunked=>IntSetRepr::Chunked,_=>unreachable!()};
                        assert_eq!(r.representation(),repr);
                    }
                }
                checks+=1;
            }}}
        }
    }}}}
    println!("{checks} forced/automatic result-policy and canonical-hash checks passed across all representation pairs.");
}
