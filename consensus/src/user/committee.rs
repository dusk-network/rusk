use crate::user::provisioners::{Member, Provisioners};

#[allow(unused)]
pub struct Committee {
    members: Vec<Member>,
}

#[allow(unused)]
impl Committee {
    pub fn new(_provisioners: &Provisioners, _round: u64, _step: u8, _seed: [u8; 32]) -> Self {
        //TODO: run sortition
        Self { members: vec![] }
    }

    pub fn is_member() -> bool {
        //TODO:
        false
    }

    pub fn am_member() -> bool {
        //TODO:
        false
    }
}
