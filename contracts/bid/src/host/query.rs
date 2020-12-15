use crate::{ops, Contract};
use canonical_host::{MemStore, Query};
use dusk_blindbid::bid::Bid;
use dusk_bls12_381::BlsScalar;

type QueryIndex = u16;

impl Contract<MemStore> {
    pub fn find_bid() -> Query<(QueryIndex, u64), [Bid; 2]> {
        //Query::new((ops::GET_LEAF, pos as u64))
        unimplemented!()
    }
}
