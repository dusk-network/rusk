use consensus::contract_state::{CallParams, Error, Operations, Output, StateRoot};

pub struct Executor {}
impl Operations for Executor {
    fn verify_state_transition(&self, _params: CallParams) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }

    fn execute_state_transition(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }
}
