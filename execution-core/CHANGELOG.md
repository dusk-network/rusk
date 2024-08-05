# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add, types, type-alias, functionality, re-exports and modules:
```rust
dusk;
from_dusk;
Dusk;
LUX;
pub use dusk_bls12_381::BlsScalar;
pub use dusk_jubjub::{
    JubJubAffine,
    JubJubExtended,
    JubJubScalar,
    GENERATOR_EXTENDED,
    GENERATOR_NUMS_EXTENDED
};
pub use piecrust_uplink::{
    ContractError,
    ContractId,
    Event,
    StandardBufSerializer,
    ARGBUF_LEN,
    CONTRACT_ID_BYTES,
};
signatures::{
    bls::{
        Error,
        PublicKey,
        SecretKey,
        Signature,
        MutlisigPublicKey,
        MultisigSignature
    };
    schnorr::{
        PublicKey,
        SecretKey,
        Signature,
        SignatureDouble,
    }
}
transfer::{
    contract_exec::{
        ContractBytecode;
        ContractCall;
        ContractExec;
    };
    moonlight::{
        AccountData;
        Payload;
        Transaction;
    };
    phoenix::{
        Fee;
        Payload;
        Transaction;
        TreeLeaf;
        NOTES_TREE_DEPTH;
        TRANSCRIPT_LABEL;
        pub use phoenix_core::{
            value_commitment,
            Error as PhoenixError,
            Note,
            PublicKey,
            SecretKey,
            Sender,
            StealthAddress,
            TxSkeleton,
            ViewKey,
            NOTE_VAL_ENC_SIZE,
            OUTPUT_NOTES
        };
        pub type NotePublicKey = SchnorrPublicKey;
        pub type NoteSecretKey = SchnorrSecretKey;
        pub type NoteSignature = SchnorrSignature;
    };
    withdraw::{
        Withdraw;
        WithdrawReceiver;
        WithdrawSignature;
        WithdrawSecretKey;
        WithdrawReplayToken;
    };
    Transaction;
    TRANSFER_CONTRACT;
};
stake::{
    Stake;
    StakeAmount;
    StakeData;
    StakeEvent;
    Withdraw;
    EPOCH;
    STAKE_CONTRACT;
    STAKE_WARNINGS;
    next_epoch;
};
licence::LICENSE_CONTRACT;
```
- under the `"zk"` feature:
```rust
plonk::{
    pub use dusk_plonk::{
        Circuit,
        Compiler,
        Composer,
        Constraint,
        Error,
        Proof,
        Prover,
        PublicParameters,
        Verifier,
        Witness,
        WitnessPoint,
    }
};
transfer::phoenix::{
    pub use phoenix_circuits{
        TxCircuit,
        TxInputNote,
        TxOutputNote,
    };
};
```

[Unreleased]: https://github.com/dusk-network/rusk/compare/execution-core-0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/dusk-abi/releases/tag/execution-core-0.1.0
