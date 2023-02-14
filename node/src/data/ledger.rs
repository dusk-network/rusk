// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::{Error as BytesError, HexDebug, Serializable};
use std::io::{self, Read, Write};

#[derive(Debug)]
pub struct Transaction {
    pub inner: dusk_wallet_core::Transaction,
    pub gas_spent: Option<u64>,
}

impl Transaction {
    pub fn hash(&self) -> [u8; 32] {
        self.inner.hash().to_bytes().into()
    }

    pub fn gas_price(&self) -> u64 {
        self.inner.fee().gas_price
    }
}

impl dusk_consensus::messages::Serializable for Transaction {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let data = self.inner.to_var_bytes();

        // Write inner transaction
        let len: u32 = data.len() as u32;
        w.write_all(&len.to_le_bytes());
        w.write_all(&data)?;

        // Write gas_spent
        match self.gas_spent {
            Some(gas_spent) => {
                w.write_all(&1_u8.to_le_bytes())?;
                w.write_all(&gas_spent.to_le_bytes())?;
            }
            None => {
                w.write_all(&0_u8.to_le_bytes())?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;

        let len = u32::from_le_bytes(buf);
        let mut buf = vec![0u8; len as usize];
        r.read_exact(&mut buf)?;

        let inner = dusk_wallet_core::Transaction::from_slice(&buf[..])
            .map_err(|err| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut optional = [0u8; 1];
        r.read_exact(&mut optional)?;

        let gas_spent = if optional[0] != 0 {
            let mut buf = [0u8; 8];
            r.read_exact(&mut buf)?;

            Some(u64::from_le_bytes(buf))
        } else {
            None
        };

        Ok(Self { inner, gas_spent })
    }
}
