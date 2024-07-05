# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Change payload to support contract deployment [#1882]


- Re-export
  - `dusk-bls12_381::BlsScalar`
  - `dusk-jubjub::{
      JubJubAffine,
      JubJubExtended,
      JubJubScalar,
      GENERATOR_EXTENDED,
      GENERATOR_NUMS_EXTENDED
    }`
  - `bls12_381_bls::{
      Error as BlsSigError,
      PublicKey as BlsPublicKey,
      SecretKey as BlsSecretKey,
      Signature as BlsSignature,
      APK as BlsAggPublicKey
    }`
  - `jubjub_schnorr::{
      PublicKey as SchnorrPublicKey,
      SecretKey as SchnorrSecretKey,
      Signature as SchnorrSignature,
      SignatureDouble as SchnorrSignatureDouble
    }`
  - `phoenix_core::{
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
    }`
- Add type-alias:
  - `pub type StakeSecretKey = BlsSecretKey`
  - `pub type StakePublicKey = BlsPublicKey`
  - `pub type StakeSignature = BlsSignature`
  - `pub type StakeAggPublicKey = BlsAggPublicKey`
  - `pub type NoteSecretKey = SchnorrSecretKey`
  - `pub type NotePublicKey = SchnorrPublicKey`
  - `pub type NoteSignature = SchnorrSignature`
- Add modules, types and functionality:
  - `transfer::{
      ContractId,
      TRANSFER_TREE_DEPTH,
      TreeLeaf,
      Mint,
      ContractCall,
      Fee,
      SenderAccount,
      Payload,
      Transaction,
    }`
  - `stake::{
      EPOCH,
      STAKE_WARNINGS,
      next_epoch,
      Stake,
      Unstake,
      Withdraw,
      StakingEvent,
      StakeData,
  }`

[#1882]: https://github.com/dusk-network/rusk/issues/1882

[Unreleased]: https://github.com/dusk-network/rusk/compare/execution-core-0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/dusk-abi/releases/tag/execution-core-0.1.0
