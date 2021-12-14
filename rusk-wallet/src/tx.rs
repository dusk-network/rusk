use alloc::vec::Vec;

use dusk_abi::ContractId;
use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use dusk_jubjub::BlsScalar;
use dusk_schnorr::Proof;
use phoenix_core::{Crossover, Fee, Note};

/// The structure sent over the network representing a transaction.
pub struct Transaction {
    inputs: Vec<BlsScalar>,
    outputs: Vec<Note>,

    anchor: BlsScalar,
    fee: Fee,
    proof: Proof,

    crossover: Option<Crossover>,
    call: Option<(ContractId, Vec<u8>)>,
}

impl Transaction {
    /// Serializes the transaction into a variable length byte buffer.
    pub fn to_var_bytes(&self) -> Result<Vec<u8>, BytesError> {
        // compute the serialized size to preallocate space
        let size = {
            let mut size = u64::SIZE
                + self.inputs.len() * BlsScalar::SIZE
                + u64::SIZE
                + self.outputs.len() * Note::SIZE
                + BlsScalar::SIZE
                + Fee::SIZE
                + Proof::SIZE
                + u64::SIZE
                + u64::SIZE;

            if self.crossover.is_some() {
                size += Crossover::SIZE;
            }

            if let Some((_, cdata)) = &self.call {
                size += 4 + cdata.len();
            }

            size
        };
        let mut bytes = vec![0u8; size];
        let mut writer = &mut bytes[..];

        writer.write(&(self.inputs.len() as u64).to_bytes())?;
        for input in &self.inputs {
            writer.write(&input.to_bytes())?;
        }

        writer.write(&(self.outputs.len() as u64).to_bytes())?;
        for output in &self.outputs {
            writer.write(&output.to_bytes())?;
        }

        writer.write(&self.anchor.to_bytes())?;
        writer.write(&self.fee.to_bytes())?;
        writer.write(&self.proof.to_bytes())?;

        match &self.crossover {
            None => {
                writer.write(&0_u64.to_bytes())?;
            }
            Some(c) => {
                writer.write(&1_u64.to_bytes())?;
                writer.write(&c.to_bytes())?;
            }
        }

        match &self.call {
            None => {
                writer.write(&0_u64.to_bytes())?;
            }
            Some((cid, cdata)) => {
                writer.write(&1_u64.to_bytes())?;
                writer.write(cid.as_bytes())?;
                writer.write(cdata)?;
            }
        }

        Ok(bytes)
    }

    /// Deserializes the transaction from a bytes buffer.
    pub fn from_bytes(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buffer = buf;

        let ninputs = u64::from_reader(&mut buffer)? as usize;
        let mut inputs = Vec::with_capacity(ninputs);

        for _ in 0..ninputs {
            inputs.push(BlsScalar::from_reader(&mut buffer)?);
        }

        let noutputs = u64::from_reader(&mut buffer)? as usize;
        let mut outputs = Vec::with_capacity(noutputs);

        for _ in 0..noutputs {
            outputs.push(Note::from_reader(&mut buffer)?);
        }

        let anchor = BlsScalar::from_reader(&mut buffer)?;
        let fee = Fee::from_reader(&mut buffer)?;
        let proof = Proof::from_reader(&mut buffer)?;

        let mut crossover = None;
        if u64::from_reader(&mut buffer)? != 0 {
            crossover = Some(Crossover::from_reader(&mut buffer)?);
        }

        let mut call = None;
        if u64::from_reader(&mut buffer)? != 0 {
            let buf_len = buffer.len();

            // needs to be at least the size of a contract ID and have some call
            // data.
            if buf_len < 5 {
                return Err(BytesError::BadLength {
                    found: buf_len,
                    expected: 5,
                });
            }

            let contract_id = ContractId::from(&buffer[0..4]);
            let call_data = Vec::from(&buffer[4..]);
            call = Some((contract_id, call_data));
        }

        Ok(Self {
            inputs,
            outputs,
            anchor,
            fee,
            crossover,
            call,
            proof,
        })
    }
}
