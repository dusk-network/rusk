// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// allow variable names like `stake_sk` and `stake_pk`.
#![allow(clippy::similar_names)]

use std::fmt::Debug;

use dusk_core::signatures::bls::PublicKey as BlsPublicKey;
use dusk_core::stake::StakeFundOwner;
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::phoenix::PublicKey as PhoenixPublicKey;
use dusk_core::transfer::Transaction;
use rand::rngs::StdRng;
use rand::SeedableRng;
use wallet_core::transaction::{
    moonlight, moonlight_deployment, moonlight_stake, moonlight_stake_reward,
    moonlight_to_phoenix, moonlight_unstake, phoenix, phoenix_deployment,
    phoenix_stake, phoenix_stake_reward, phoenix_to_moonlight, phoenix_unstake,
};
use zeroize::Zeroize;

use super::{Address, SecureWalletFile, Wallet};
use crate::clients::Prover;
use crate::currency::Dusk;
use crate::gas::Gas;
use crate::Error;

impl<F: SecureWalletFile + Debug> Wallet<F> {
    /// Transfers funds between shielded accounts.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `sender_idx` doesn't exist within the wallet, if the transfer
    /// amount is 0 or the specified gas is not enough..
    pub async fn phoenix_transfer(
        &self,
        sender_idx: u8,
        receiver_pk: &PhoenixPublicKey,
        memo: Option<String>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 && memo.is_none() {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;

        let mut rng = StdRng::from_entropy();
        let amt = *amt;

        let refund_pk = self.shielded_key(sender_idx)?;

        let tx_cost = amt + gas.limit * gas.price;
        let inputs = state
            .tx_input_notes(sender_idx, tx_cost)
            .await?
            .into_iter()
            .map(|(note, opening, _nullifier)| (note, opening))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_phoenix_sk(sender_idx);

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            refund_pk,
            receiver_pk,
            inputs,
            root,
            amt,
            true,
            0,
            gas.limit,
            gas.price,
            chain_id,
            memo,
            &Prover,
        );

        sender_sk.zeroize();

        let tx = state.prove(tx?).await?;
        state.propagate(tx).await
    }

    /// Transfers funds between public accounts.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `sender_idx` doesn't exist within the wallet, if the transfer
    /// amount is 0 or the specified gas is not enough..
    pub async fn moonlight_transfer(
        &self,
        sender_idx: u8,
        rcvr: &BlsPublicKey,
        memo: Option<String>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 && memo.is_none() {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let sender_pk = self.public_key(sender_idx)?;
        let amt = *amt;

        let state = self.state()?;
        let nonce = state.fetch_account(sender_pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_bls_sk(sender_idx);

        let tx = moonlight(
            &sender_sk,
            Some(*rcvr),
            amt,
            0,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            memo,
        );

        sender_sk.zeroize();

        state.propagate(tx?).await
    }

    /// Executes a generic contract call, paying gas with a shielded account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    pub async fn phoenix_execute(
        &self,
        sender_idx: u8,
        deposit: Dusk,
        gas: Gas,
        data: TransactionData,
    ) -> Result<Transaction, Error> {
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let mut rng = StdRng::from_entropy();
        // in a contract execution or deployment, the sender and receiver are
        // the same
        let receiver_pk = self.shielded_key(sender_idx)?;

        let tx_cost = deposit + gas.limit * gas.price;
        let inputs = state
            .tx_input_notes(sender_idx, tx_cost)
            .await?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_phoenix_sk(sender_idx);

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            self.shielded_key(sender_idx)?,
            receiver_pk,
            inputs,
            root,
            0,
            true,
            deposit,
            gas.limit,
            gas.price,
            chain_id,
            Some(data),
            &Prover,
        );

        sender_sk.zeroize();

        let tx = state.prove(tx?).await?;
        state.propagate(tx).await
    }

    /// Executes a generic contract call, paying gas from a public account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    #[allow(clippy::too_many_arguments)]
    pub async fn moonlight_execute(
        &self,
        sender_idx: u8,
        transfer_value: Dusk,
        deposit: Dusk,
        gas: Gas,
        exec: Option<impl Into<TransactionData>>,
    ) -> Result<Transaction, Error> {
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let sender = self.public_key(sender_idx)?;

        let account = state.fetch_account(sender).await?;

        // technically this check is not necessary, but it's nice to not spam
        // the network with transactions that are unspendable.
        let nonce = account.nonce + 1;

        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_bls_sk(sender_idx);

        let tx = moonlight(
            &sender_sk,
            None,
            *transfer_value,
            deposit,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            exec,
        );

        sender_sk.zeroize();

        state.propagate(tx?).await
    }

    /// Stakes Dusk using shielded notes.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `profile_idx` doesn't exist within the wallet, the stake-amount
    /// is 0 or the gas is insufficient.
    pub async fn phoenix_stake(
        &self,
        profile_idx: u8,
        owner_idx: Option<u8>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check if the gas is enough
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;

        let mut rng = StdRng::from_entropy();
        let amt = *amt;

        let stake_pk = self.public_key(profile_idx)?;
        let stake_owner_idx = match self.find_stake_owner_idx(stake_pk).await {
            Ok(state_idx) => {
                if let Some(owner_idx) = owner_idx {
                    if state_idx != owner_idx {
                        return Err(Error::Unauthorized);
                    }
                }
                state_idx
            }
            Err(Error::NotStaked) => owner_idx.unwrap_or(profile_idx),
            Err(e) => {
                return Err(e);
            }
        };

        let tx_cost = amt + gas.limit * gas.price;
        let inputs = state
            .tx_input_notes(profile_idx, tx_cost)
            .await?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_phoenix_sk(profile_idx);
        let mut stake_sk = self.derive_bls_sk(profile_idx);
        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);

        let stake = phoenix_stake(
            &mut rng,
            &sender_sk,
            &stake_sk,
            &stake_owner_sk,
            inputs,
            root,
            gas.limit,
            gas.price,
            chain_id,
            amt,
            &Prover,
        );

        sender_sk.zeroize();
        stake_sk.zeroize();
        stake_owner_sk.zeroize();

        let stake = state.prove(stake?).await?;
        state.propagate(stake).await
    }

    /// Stakes Dusk using a public account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `profile_idx` doesn't exist within the wallet, the stake-amount
    /// is 0 or the gas is insufficient.
    pub async fn moonlight_stake(
        &self,
        profile_idx: u8,
        owner_idx: Option<u8>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check if the gas is enough
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let amt = *amt;
        let stake_pk = self.public_key(profile_idx)?;
        let chain_id = state.fetch_chain_id().await?;
        let moonlight_current_nonce =
            state.fetch_account(stake_pk).await?.nonce + 1;

        let stake_owner_idx = match self.find_stake_owner_idx(stake_pk).await {
            Ok(state_idx) => {
                if let Some(owner_idx) = owner_idx {
                    if state_idx != owner_idx {
                        return Err(Error::Unauthorized);
                    }
                }
                state_idx
            }
            Err(Error::NotStaked) => owner_idx.unwrap_or(profile_idx),
            Err(e) => {
                return Err(e);
            }
        };

        let mut stake_sk = self.derive_bls_sk(profile_idx);
        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);

        let stake = moonlight_stake(
            &stake_sk,
            &stake_sk,
            &stake_owner_sk,
            amt,
            gas.limit,
            gas.price,
            moonlight_current_nonce,
            chain_id,
        );

        stake_sk.zeroize();
        stake_owner_sk.zeroize();

        state.propagate(stake?).await
    }

    /// Unstakes Dusk into shielded notes.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `profile_idx` doesn't exist within the wallet or there is no
    /// stake for the given profile.
    pub async fn phoenix_unstake(
        &self,
        profile_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();

        let state = self.state()?;

        let tx_cost = gas.limit * gas.price;
        let inputs = state.tx_input_notes(profile_idx, tx_cost).await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut stake_sk = self.derive_bls_sk(profile_idx);
        let stake_pk = BlsPublicKey::from(&stake_sk);

        let stake_owner_idx = self
            .find_stake_owner_idx(&stake_pk)
            .await
            .inspect_err(|_| stake_sk.zeroize())?;

        let unstake_value = state
            .fetch_stake(&stake_pk)
            .await
            .inspect_err(|_| stake_sk.zeroize())?
            .and_then(|s| s.amount)
            .map(|s| s.total_funds())
            .unwrap_or_default();

        if unstake_value == 0 {
            stake_sk.zeroize();
            return Err(Error::NotStaked);
        }

        let mut sender_sk = self.derive_phoenix_sk(profile_idx);
        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);

        let unstake = phoenix_unstake(
            &mut rng,
            &sender_sk,
            &stake_sk,
            &stake_owner_sk,
            inputs,
            root,
            unstake_value,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        );

        sender_sk.zeroize();
        stake_sk.zeroize();
        stake_owner_sk.zeroize();

        let unstake = state.prove(unstake?).await?;
        state.propagate(unstake).await
    }

    /// Unstakes Dusk onto a public account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if the `profile_idx` doesn't exist within the wallet or there is no
    /// stake for the given profile.
    pub async fn moonlight_unstake(
        &self,
        profile_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let stake_pk = self.public_key(profile_idx)?;

        let chain_id = state.fetch_chain_id().await?;
        let account_nonce = state.fetch_account(stake_pk).await?.nonce + 1;

        let unstake_value = state
            .fetch_stake(stake_pk)
            .await?
            .and_then(|s| s.amount)
            .map(|s| s.total_funds())
            .unwrap_or_default();

        if unstake_value == 0 {
            return Err(Error::NotStaked);
        }

        let stake_owner_idx = self.find_stake_owner_idx(stake_pk).await?;

        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);
        let mut stake_sk = self.derive_bls_sk(profile_idx);

        let unstake = moonlight_unstake(
            &mut rng,
            &stake_sk,
            &stake_sk,
            &stake_owner_sk,
            unstake_value,
            gas.limit,
            gas.price,
            account_nonce,
            chain_id,
        );

        stake_sk.zeroize();
        stake_owner_sk.zeroize();

        state.propagate(unstake?).await
    }

    /// Withdraws accumulated staking to a shielded account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    pub async fn phoenix_claim_rewards(
        &self,
        sender_idx: u8,
        reward_amt: Option<Dusk>,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let state = self.state()?;
        let mut rng = StdRng::from_entropy();

        let tx_cost = gas.limit * gas.price;
        let inputs = state.tx_input_notes(sender_idx, tx_cost).await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let available_reward = self.get_stake_reward(sender_idx).await?;

        let reward_amt_claimed = match reward_amt {
            // if user specified an amount to claim, we check if it's
            // available
            Some(reward_amt) => {
                if reward_amt > available_reward {
                    return Err(Error::NotEnoughReward);
                }
                *reward_amt
            }
            // claim all the rewards if no amt specified
            None => *available_reward,
        };

        let mut stake_sk = self.derive_bls_sk(sender_idx);
        let stake_pk = BlsPublicKey::from(&stake_sk);

        let stake_owner_idx = self
            .find_stake_owner_idx(&stake_pk)
            .await
            .inspect_err(|_| stake_sk.zeroize())?;

        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);
        let mut sender_sk = self.derive_phoenix_sk(sender_idx);

        let claim_rewards = phoenix_stake_reward(
            &mut rng,
            &sender_sk,
            &stake_sk,
            &stake_owner_sk,
            inputs,
            root,
            reward_amt_claimed,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        );

        sender_sk.zeroize();
        stake_sk.zeroize();
        stake_owner_sk.zeroize();

        let claim_rewards = state.prove(claim_rewards?).await?;
        state.propagate(claim_rewards).await
    }

    /// Withdraws accumulated staking reward to a public account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    pub async fn moonlight_claim_rewards(
        &self,
        sender_idx: u8,
        reward_amt: Option<Dusk>,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let pk = self.public_key(sender_idx)?;
        let nonce = state.fetch_account(pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;
        let available_reward = self.get_stake_reward(sender_idx).await?;

        let reward_amt_claimed = match reward_amt {
            // if user specified an amount to claim we check if it's
            // available
            Some(reward_amt) => {
                if reward_amt > available_reward {
                    return Err(Error::NotEnoughReward);
                }
                *reward_amt
            }
            // claim all the reward if no amt specified
            None => *available_reward,
        };

        let stake_pk = self.public_key(sender_idx)?;
        let stake_owner_idx = self.find_stake_owner_idx(stake_pk).await?;

        let mut sender_sk = self.derive_bls_sk(sender_idx);
        let mut stake_owner_sk = self.derive_bls_sk(stake_owner_idx);

        let claim_rewards = moonlight_stake_reward(
            &mut rng,
            &sender_sk,
            &sender_sk,
            &stake_owner_sk,
            reward_amt_claimed,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
        );

        sender_sk.zeroize();
        stake_owner_sk.zeroize();

        state.propagate(claim_rewards?).await
    }

    /// Converts Dusk from a shielded account to a public account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `profile_idx` doesn't exist within the wallet.
    pub async fn phoenix_to_moonlight(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;
        let tx_cost = *amt + gas.limit * gas.price;
        let inputs = state.tx_input_notes(profile_idx, tx_cost).await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut phoenix_sk = self.derive_phoenix_sk(profile_idx);
        let mut moonlight_sk = self.derive_bls_sk(profile_idx);

        let convert = phoenix_to_moonlight(
            &mut rng,
            &phoenix_sk,
            &moonlight_sk,
            inputs,
            root,
            *amt,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        );

        phoenix_sk.zeroize();
        moonlight_sk.zeroize();

        let convert = state.prove(convert?).await?;
        state.propagate(convert).await
    }

    /// Converts Dusk from a public account to a shielded account.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `profile_idx` doesn't exist within the wallet.
    pub async fn moonlight_to_phoenix(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let moonlight_pk = self.public_key(profile_idx)?;

        let nonce = state.fetch_account(moonlight_pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let mut phoenix_sk = self.derive_phoenix_sk(profile_idx);
        let mut moonlight_sk = self.derive_bls_sk(profile_idx);

        let convert = moonlight_to_phoenix(
            &mut rng,
            &moonlight_sk,
            &phoenix_sk,
            *amt,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
        );

        phoenix_sk.zeroize();
        moonlight_sk.zeroize();

        state.propagate(convert?).await
    }

    /// Deploys a contract using shielded notes to pay gas.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    pub async fn phoenix_deploy(
        &self,
        sender_idx: u8,
        bytes_code: Vec<u8>,
        init_args: Vec<u8>,
        deploy_nonce: u64,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let chain_id = state.fetch_chain_id().await?;
        let root = state.fetch_root().await?;

        let tx_cost = gas.limit * gas.price;
        let inputs = state.tx_input_notes(sender_idx, tx_cost).await?;

        let owner_pk = self.public_key(sender_idx)?;
        let mut sender_sk = self.derive_phoenix_sk(sender_idx);

        let deploy = phoenix_deployment(
            &mut rng,
            &sender_sk,
            inputs,
            root,
            bytes_code,
            owner_pk,
            init_args,
            deploy_nonce,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        );

        sender_sk.zeroize();

        let deploy = state.prove(deploy?).await?;
        state.propagate(deploy).await
    }

    /// Deploys a contract using a public account to pay gas.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if the `sender_idx` doesn't exist within the wallet.
    pub async fn moonlight_deploy(
        &self,
        sender_idx: u8,
        bytes_code: Vec<u8>,
        init_args: Vec<u8>,
        deploy_nonce: u64,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let state = self.state()?;

        let pk = self.public_key(sender_idx)?;
        let moonlight_nonce = state.fetch_account(pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_bls_sk(sender_idx);

        let deploy = moonlight_deployment(
            &sender_sk,
            bytes_code,
            pk,
            init_args,
            gas.limit,
            gas.price,
            moonlight_nonce,
            deploy_nonce,
            chain_id,
        );

        sender_sk.zeroize();

        state.propagate(deploy?).await
    }

    /// Finds the index of the stake owner account.
    ///
    /// # Errors
    /// This method will error if the given `stake_pk` cannot be found in the
    /// wallet.
    pub async fn find_stake_owner_idx(
        &self,
        stake_pk: &BlsPublicKey,
    ) -> Result<u8, Error> {
        self.find_index(&self.find_stake_owner_account(stake_pk).await?)
    }

    /// Finds the address of the stake owner account.
    ///
    /// # Errors
    /// This method will error if the stake-owner cannot be fetched or if the
    /// stake-owner of the provided `stake_pk` is a contract.
    pub async fn find_stake_owner_account(
        &self,
        stake_pk: &BlsPublicKey,
    ) -> Result<Address, Error> {
        let stake_owner = self
            .state()?
            .fetch_stake_owner(stake_pk)
            .await?
            .ok_or(Error::NotStaked)?;

        match stake_owner {
            StakeFundOwner::Account(public_key) => {
                Ok(Address::Public(public_key))
            }
            StakeFundOwner::Contract(_) => Err(Error::Unauthorized),
        }
    }
}
