// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

#[cfg(test)]
mod transaction_tests {
    use dusk_pki::PublicSpendKey;
    use dusk_plonk::bls12_381::Scalar as BlsScalar;
    use dusk_plonk::jubjub::{
        AffinePoint as JubJubAffine, ExtendedPoint as JubJubExtended,
        Fr as JubJubScalar, GENERATOR_EXTENDED,
    };
    use phoenix_core::Note;
    use poseidon252::cipher::PoseidonCipher;
    use rand::Rng;
    use rusk::services::rusk_proto;
    use rusk::tx::{Crossover, Fee, Transaction};
    use std::convert::TryInto;
    use std::io::{Read, Write};

    fn random_note() -> Note {
        let t: u8 = rand::thread_rng().gen_range(0, 2);

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let a = JubJubExtended::from(JubJubAffine::from(a));
        let b = JubJubExtended::from(JubJubAffine::from(b));

        let pk = PublicSpendKey::new(a, b);

        let value: u64 = rand::thread_rng().gen();

        Note::new(t.try_into().unwrap(), &pk, value)
    }

    fn random_fee() -> Fee {
        let gas_limit: u64 = rand::thread_rng().gen();
        let gas_price: u64 = rand::thread_rng().gen();

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let a = GENERATOR_EXTENDED * s;

        let s = JubJubScalar::random(&mut rand::thread_rng());
        let b = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let a = JubJubExtended::from(JubJubAffine::from(a));
        let b = JubJubExtended::from(JubJubAffine::from(b));

        let address = PublicSpendKey::new(a, b).gen_stealth_address(
            &JubJubScalar::random(&mut rand::thread_rng()),
        );

        Fee::new(gas_limit, gas_price, address)
    }

    fn random_crossover() -> Crossover {
        let s = JubJubScalar::random(&mut rand::thread_rng());
        let value_commitment = GENERATOR_EXTENDED * s;

        // We need to cast extended points to affine and back,
        // to ensure integrity of data.
        let value_commitment =
            JubJubExtended::from(JubJubAffine::from(value_commitment));

        let nonce = BlsScalar::random(&mut rand::thread_rng());

        let scalars = [BlsScalar::random(&mut rand::thread_rng()); 3];
        let encrypted_data = PoseidonCipher::new(scalars);

        Crossover::new(value_commitment, nonce, encrypted_data)
    }

    fn random_tx() -> Transaction {
        // Create a transaction with randomised fields
        // NOTE: it is a bit tough to make a random proof,
        // so we will leave this out of the test for now.
        let mut tx = Transaction::default();

        let t = rand::thread_rng().gen_range(0, 8);
        tx.set_type(t.try_into().unwrap());

        tx.mut_payload()
            .set_anchor(BlsScalar::random(&mut rand::thread_rng()));

        let num_nuls = rand::thread_rng().gen_range(1, 4);
        for _ in 0..num_nuls {
            tx.mut_payload()
                .add_nullifier(BlsScalar::random(&mut rand::thread_rng()));
        }

        let num_notes = rand::thread_rng().gen_range(1, 2);
        for _ in 0..num_notes {
            tx.mut_payload().add_note(random_note());
        }

        tx.mut_payload().set_fee(random_fee());

        tx.mut_payload().set_crossover(random_crossover());

        let call_data_size = rand::thread_rng().gen_range(100, 1000);
        let call_data: Vec<u8> = (0..call_data_size)
            .map(|_| rand::thread_rng().gen::<u8>())
            .collect();

        tx.mut_payload().set_call_data(call_data);
        tx
    }

    // Ensure that a transaction stays the same when encoded to
    // and decoded from protocol buffers.
    #[test]
    fn transaction_encode_decode() {
        let tx = random_tx();
        let pbuf_tx: rusk_proto::Transaction = tx.clone().try_into().unwrap();
        let decoded_tx: Transaction = (&pbuf_tx).try_into().unwrap();

        assert_eq!(tx, decoded_tx);
    }

    #[test]
    fn transaction_read_write() {
        let mut tx = random_tx();

        let mut buf = [0u8; 2048];
        tx.read(&mut buf).unwrap();

        let mut decoded_tx = Transaction::default();
        decoded_tx.write(&mut buf).unwrap();

        assert_eq!(tx, decoded_tx);
    }
}
