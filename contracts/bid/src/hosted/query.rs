use crate::Contract;
use canonical::Store;
use dusk_bls12_381::BlsScalar;

impl<S: Store> Contract<S> {
    pub fn find_bid(&self, idx: u64) -> () {
        unimplemented!()
    }
}
