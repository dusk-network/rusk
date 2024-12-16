// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::tree::Tree;
use crate::verifier_data::tx_circuit_verifier;

use alloc::collections::btree_map::Entry;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use dusk_core::stake::EPOCH;
use dusk_core::transfer::MINT_CONTRACT_TOPIC;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

use dusk_core::{
    signatures::bls::PublicKey as AccountPublicKey,
    stake::STAKE_CONTRACT,
    transfer::{
        moonlight::{AccountData, Transaction as MoonlightTransaction},
        phoenix::{
            Note, NoteLeaf, NoteOpening, Sender,
            Transaction as PhoenixTransaction,
        },
        withdraw::{
            Withdraw, WithdrawReceiver, WithdrawReplayToken, WithdrawSignature,
        },
        ContractToAccount, ContractToAccountEvent, ContractToContract,
        ContractToContractEvent, ConvertEvent, DepositEvent,
        MoonlightTransactionEvent, PhoenixTransactionEvent,
        ReceiveFromContract, Transaction, WithdrawEvent,
        CONTRACT_TO_ACCOUNT_TOPIC, CONTRACT_TO_CONTRACT_TOPIC, CONVERT_TOPIC,
        DEPOSIT_TOPIC, MINT_TOPIC, MOONLIGHT_TOPIC, PANIC_NONCE_NOT_READY,
        PHOENIX_TOPIC, TRANSFER_CONTRACT, WITHDRAW_TOPIC,
    },
    BlsScalar, ContractError, ContractId,
};

use crate::transitory;
use transitory::Deposit;

/// Number of roots stored
pub const MAX_ROOTS: usize = 2 * EPOCH as usize;

/// An empty account, used as the default return and for instantiating new
/// entries.
const EMPTY_ACCOUNT: AccountData = AccountData {
    nonce: 0,
    balance: 0,
};

fn contract_fn_sender(fn_name: &str, contract: ContractId) -> Sender {
    let mut bytes = [0u8; 128];

    let mut offset = 0;

    bytes[offset..offset + fn_name.len()].copy_from_slice(fn_name.as_bytes());
    offset += fn_name.len();

    bytes[offset..offset + 32].copy_from_slice(&contract.to_bytes());

    Sender::ContractInfo(bytes)
}

pub struct TransferState {
    tree: Tree,
    nullifiers: BTreeSet<BlsScalar>,
    roots: ConstGenericRingBuffer<BlsScalar, MAX_ROOTS>,
    // NOTE: we should never remove entries from this list, since the entries
    //       contain the nonce of the given account. Doing so opens the account
    //       up to replay attacks.
    accounts: BTreeMap<[u8; 193], AccountData>,
    contract_balances: BTreeMap<ContractId, u64>,
}

impl TransferState {
    pub const fn new() -> TransferState {
        TransferState {
            tree: Tree::new(),
            nullifiers: BTreeSet::new(),
            roots: ConstGenericRingBuffer::new(),
            accounts: BTreeMap::new(),
            contract_balances: BTreeMap::new(),
        }
    }

    /// Checks the [`Withdraw`] is correct, and mints the amount of the
    /// withdrawal.
    fn mint_withdrawal(&mut self, fn_name: &str, withdraw: &Withdraw) {
        let contract = withdraw.contract();
        let value = withdraw.value();

        let msg = withdraw.signature_message();
        let signature = withdraw.signature();

        match withdraw.token() {
            WithdrawReplayToken::Phoenix(nullifiers) => {
                let phoenix_tx = transitory::phoenix_transaction();

                for n in phoenix_tx.nullifiers() {
                    if !nullifiers.contains(n) {
                        panic!("Incorrect nullifiers signed");
                    }
                }
            }
            WithdrawReplayToken::Moonlight(nonce) => {
                let moonlight_tx = transitory::moonlight_transaction();

                if *nonce != moonlight_tx.nonce() {
                    panic!("Incorrect nonce signed");
                }
            }
        }

        match withdraw.receiver() {
            WithdrawReceiver::Phoenix(address) => {
                let signature = match signature {
                    WithdrawSignature::Phoenix(s) => s,
                    _ => panic!(
                        "Withdrawal to Phoenix must be signed with Schnorr"
                    ),
                };

                let hash = rusk_abi::hash(msg);
                let pk = address.note_pk();

                if !rusk_abi::verify_schnorr(hash, *pk, *signature) {
                    panic!("Invalid signature");
                }

                let sender = contract_fn_sender(fn_name, *contract);

                let note = Note::transparent_stealth(*address, value, sender);
                self.push_note_current_height(note);
            }
            WithdrawReceiver::Moonlight(account) => {
                let signature = match signature {
                    WithdrawSignature::Moonlight(s) => s,
                    _ => panic!(
                        "Withdrawal to Moonlight must be signed with BLS"
                    ),
                };

                if !rusk_abi::verify_bls(msg, *account, *signature) {
                    panic!("Invalid signature");
                }

                let account_bytes = account.to_raw_bytes();
                let account =
                    self.accounts.entry(account_bytes).or_insert(EMPTY_ACCOUNT);

                account.balance += value;
            }
        }
    }

    /// Mint more Dusk.
    ///
    /// This can only be called by the stake contract, and will increase the
    /// total amount of circulating Dusk. It is intended to be called during the
    /// execution of the `withdraw` function, and the amount minted should
    /// conform to the consensus emission schedule.
    ///
    /// # Safety
    /// We assume on trust that the value sent by the stake contract is
    /// according to consensus rules.
    pub fn mint(&mut self, mint: Withdraw) {
        const PANIC_MSG: &str = "Can only be called by the stake contract";
        if rusk_abi::caller().expect(PANIC_MSG) != STAKE_CONTRACT {
            panic!("{PANIC_MSG}")
        }

        if mint.contract() != &STAKE_CONTRACT {
            panic!("Withdrawal should from the stake contract");
        }

        self.mint_withdrawal("mint", &mint);

        rusk_abi::emit(MINT_TOPIC, WithdrawEvent::from(mint));
    }

    /// Mint more Dusk to be owned by a contract.
    ///
    /// This can only be called by the stake contract, and will increase the
    /// total amount of circulating Dusk. It is intended to be called during the
    /// execution of the `withdraw_to_contract` function, and the amount minted
    /// should conform to the consensus emission schedule.
    ///
    /// # Safety
    /// We assume on trust that the value sent by the stake contract is
    /// according to consensus rules.
    pub fn mint_to_contract(&mut self, mint: ContractToContract) {
        const PANIC_MSG: &str = "Can only be called by the stake contract";
        let caller = rusk_abi::caller().expect(PANIC_MSG);
        assert_eq!(caller, STAKE_CONTRACT, "{PANIC_MSG}");

        let receiver_balance =
            self.contract_balances.entry(mint.contract).or_insert(0);

        *receiver_balance += mint.value;

        let receive = ReceiveFromContract {
            contract: STAKE_CONTRACT,
            value: mint.value,
            data: mint.data,
        };

        rusk_abi::call::<_, ()>(mint.contract, &mint.fn_name, &receive)
            .expect("Calling receiver should succeed");

        let mint_event = ContractToContractEvent {
            sender: STAKE_CONTRACT,
            receiver: mint.contract,
            value: mint.value,
        };
        rusk_abi::emit(MINT_CONTRACT_TOPIC, mint_event);
    }

    /// Withdraw from a contract's balance to a Phoenix note or a Moonlight
    /// account.
    ///
    /// Users sign the `Withdraw` data, which the contract being called
    /// (withdrawn from) is then responsible for making available to this
    /// contract via a call to this function. The function allows for
    /// withdrawals to both Phoenix notes and Moonlight accounts.
    ///
    /// # Panics
    /// This can only be called by the contract specified, and only if said
    /// contract has enough balance.
    pub fn withdraw(&mut self, withdraw: Withdraw) {
        let contract = withdraw.contract();

        let caller = rusk_abi::caller()
            .expect("A withdrawal must happen in the context of a transaction");
        if *contract != caller {
            panic!("The \"withdraw\" function can only be called by the specified contract.");
        }

        let value = withdraw.value();

        if self.contract_balance(contract) < value {
            panic!("The contract doesn't have enough balance");
        }

        self.sub_contract_balance(contract, value)
            .expect("Subtracting balance from contract should succeed");

        self.mint_withdrawal("withdraw", &withdraw);

        rusk_abi::emit(WITHDRAW_TOPIC, WithdrawEvent::from(withdraw));
    }

    /// Takes the deposit addressed to this contract, and immediately withdraws
    /// it, effectively performing an atomic conversion between Phoenix notes
    /// and Moonlight balance.
    ///
    /// This functions checks whether the deposit included with the transaction
    /// is the exact value included in `convert`, and imposes that the
    /// caller is indeed this contract.
    ///
    /// # Panics
    /// This can only be called by this contract - the transfer contract - and
    /// will panic if this is not the case.
    pub fn convert(&mut self, convert: Withdraw) {
        // since each transaction only has, at maximum, a single contract call,
        // this check impliest that this is the first contract call.
        let caller = rusk_abi::caller()
            .expect("A conversion must happen in the context of a transaction");
        if caller != TRANSFER_CONTRACT {
            panic!("Only the first contract call can be a conversion");
        }

        if *convert.contract() != TRANSFER_CONTRACT {
            panic!("The conversion must target the transfer contract");
        }

        let deposit = transitory::deposit_info_mut();
        match deposit {
            Deposit::Available {
                sender,
                value: deposit_value,
                ..
            } => {
                let deposit_value = *deposit_value;

                if convert.value() != deposit_value {
                    panic!("The value to convert doesn't match the value in the transaction");
                }

                // Since this is the first contract call, and the target of a
                // deposit is always the first contract call, we can skip this
                // check.
                // if deposit_contract != TRANSFER_CONTRACT {
                //     panic!();
                // }

                // copy here because `set_taken` needs a mutable reference
                let sender = *sender;

                // Handle the withdrawal part of the conversion and set the
                // deposit as being taken. Interesting to note is that we don't
                // need to change the value held by the contract at all, since
                // it never changes.
                self.mint_withdrawal("convert", &convert);
                deposit.set_taken();

                rusk_abi::emit(
                    CONVERT_TOPIC,
                    ConvertEvent::from_withdraw_and_sender(sender, &convert),
                );
            }
            Deposit::None => panic!("There is no deposit in the transaction"),
            // Since this is the first contract call, it is impossible for the
            // deposit to be already taken.
            _ => unreachable!(),
        }
    }

    /// Deposit funds to a contract's balance.
    ///
    /// This function checks whether a deposit has been placed earlier on the
    /// state. If so and the contract-id matches the caller, the deposit will be
    /// added to the contract's balance.
    ///
    /// # Panics
    /// This function will panic if there is no deposit on the state or the
    /// caller-id doesn't match the contract-id stored for the deposit.
    pub fn deposit(&mut self, value: u64) {
        let caller = rusk_abi::caller()
            .expect("A deposit must happen in the context of a transaction");

        let deposit = transitory::deposit_info_mut();
        match deposit {
            Deposit::Available {
                sender,
                target: deposit_contract,
                value: deposit_value,
            } => {
                let deposit_contract = *deposit_contract;
                let deposit_value = *deposit_value;

                if deposit_value != value {
                    panic!(
                        "The value to deposit doesn't match the value in the transaction"
                    );
                }

                if deposit_contract != caller {
                    panic!("The calling contract doesn't match the contract in the transaction");
                }

                // copy here because `set_taken` needs a mutable reference
                let sender = *sender;

                // add to the contract's balance and set the deposit as taken
                self.add_contract_balance(deposit_contract, deposit_value);
                deposit.set_taken();

                rusk_abi::emit(
                    DEPOSIT_TOPIC,
                    DepositEvent {
                        sender,
                        value: deposit_value,
                        receiver: deposit_contract,
                    },
                );
            }
            Deposit::Taken { .. } => {
                panic!("The deposit has already been taken")
            }
            Deposit::None => panic!("There is no deposit in the transaction"),
        }
    }

    /// Transfer funds from one contract's balance to another.
    ///
    /// Contracts can call the function and expect that if it succeeds the funds
    /// are succesfully transferred to the contract they specify. Contracts
    /// receiving funds are expected to expose the function specified by the
    /// sender, which is called using a [`ReceiveFromContract`] as argument. It
    /// is recommended that the receiving contract check that the call
    /// originates from the transfer contract, and subsequently run any logic it
    /// may wish to handle the transfer - including panicking, which will
    /// effectively reject the transfer.
    ///
    /// # Panics
    /// The function will panic if it is not being called by a contract (or if
    /// it is called by the transfer contract itself), if the call to the
    /// receiving contract fails, or if the sending contract doesn't have enough
    /// funds.
    pub fn contract_to_contract(&mut self, transfer: ContractToContract) {
        let sender_contract = rusk_abi::caller()
            .expect("A transfer to a contract must happen in the context of a transaction");

        if sender_contract == TRANSFER_CONTRACT {
            panic!("Cannot be called directly by the transfer contract");
        }

        let sender_balance = self
            .contract_balances
            .get_mut(&sender_contract)
            .expect("Caller must have a balance");

        if *sender_balance < transfer.value {
            panic!("Caller must have enough balance");
        }

        *sender_balance -= transfer.value;

        let receiver_balance =
            self.contract_balances.entry(transfer.contract).or_insert(0);

        *receiver_balance += transfer.value;

        let receive = ReceiveFromContract {
            contract: sender_contract,
            value: transfer.value,
            data: transfer.data,
        };

        rusk_abi::call::<_, ()>(transfer.contract, &transfer.fn_name, &receive)
            .expect("Calling receiver should succeed");

        rusk_abi::emit(
            CONTRACT_TO_CONTRACT_TOPIC,
            ContractToContractEvent {
                sender: sender_contract,
                receiver: transfer.contract,
                value: transfer.value,
            },
        );
    }

    /// Transfer funds from a contract balance to a Moonlight account.
    ///
    /// Contracts can call the function and expect that if it succeeds the funds
    /// are successfully transferred to the account they specify.
    ///
    /// # Panics
    /// The function will panic if it is not being called by a contract, if it
    /// is called by the transfer contract itself, or if the calling contract
    /// doesn't have enough funds.
    pub fn contract_to_account(&mut self, transfer: ContractToAccount) {
        let sender_contract = rusk_abi::caller()
            .expect("A transfer to an account must happen in the context of a transaction");

        if sender_contract == TRANSFER_CONTRACT {
            panic!("Cannot be called directly by the transfer contract");
        }

        let sender_balance = self
            .contract_balances
            .get_mut(&sender_contract)
            .expect("Caller must have a balance");

        if *sender_balance < transfer.value {
            panic!("Caller must have enough balance");
        }

        let account = self
            .accounts
            .entry(transfer.account.to_raw_bytes())
            .or_insert(EMPTY_ACCOUNT);

        *sender_balance -= transfer.value;
        account.balance += transfer.value;

        rusk_abi::emit(
            CONTRACT_TO_ACCOUNT_TOPIC,
            ContractToAccountEvent {
                sender: sender_contract,
                receiver: transfer.account,
                value: transfer.value,
            },
        );
    }

    /// The top level transaction execution function.
    ///
    /// This will emplace the deposit in the state, if it exists - making it
    /// available for any contracts called.
    ///
    /// [`refund`] **must** be called if this function doesn't panic, otherwise
    /// we will have an inconsistent state.
    ///
    /// It delegate the spending phase to [`Self::spend_phoenix`] and
    /// [`Self::spend_moonlight`], depending on if the transaction
    /// uses the Phoenix or the Moonlight models, respectively.
    ///
    /// Finally executes the contract call if present.
    ///
    /// # Panics
    /// Any failure while spending will result in a panic. The contract expects
    /// the environment to roll back any change in state.
    ///
    /// [`refund`]: [`TransferState::refund`]
    pub fn spend_and_execute(
        &mut self,
        tx: Transaction,
    ) -> Result<Vec<u8>, ContractError> {
        if tx.gas_price() == 0 {
            panic!("Gas price too low!");
        }

        transitory::put_transaction(tx);
        let tx = transitory::transaction();

        match tx {
            Transaction::Phoenix(tx) => self.spend_phoenix(tx),
            Transaction::Moonlight(tx) => self.spend_moonlight(tx),
        }

        match tx.call() {
            Some(call) => {
                rusk_abi::call_raw(call.contract, &call.fn_name, &call.fn_args)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Spends the inputs and creates the given UTXO within the given phoenix
    /// transaction. It performs all checks necessary to ensure the transaction
    /// is valid - hash matches, anchor has been a root of the tree, proof
    /// checks out, etc...
    ///
    /// # Panics
    /// Any failure in the checks performed in processing the transaction will
    /// result in a panic. The contract expects the environment to roll back any
    /// change in state.
    fn spend_phoenix(&mut self, phoenix_tx: &PhoenixTransaction) {
        if phoenix_tx.chain_id() != self.chain_id() {
            panic!("The tx must target the correct chain");
        }

        // panic if the root is invalid
        if !self.root_exists(phoenix_tx.root()) {
            panic!("Root not found in the state!");
        }

        // append the nullifiers to the set, and panic if an equal one has
        // already been inserted
        for nullifier in phoenix_tx.nullifiers() {
            if !self.nullifiers.insert(*nullifier) {
                panic!("A provided nullifier has already been spent");
            }
        }

        // verify the phoenix-circuit
        if !verify_tx_proof(phoenix_tx) {
            panic!("Invalid transaction proof!");
        }

        // append the output notes to the phoenix-notes tree
        let block_height = rusk_abi::block_height();
        for note in self
            .tree
            .extend_notes(block_height, phoenix_tx.outputs().clone())
        {
            // every note is pushed to the transitory state, so it can be
            // subsequently picked up by `refund`
            transitory::push_note(note);
        }
    }

    /// Spends the amount available to the moonlight transaction. It performs
    /// all checks necessary to ensure the transaction is valid - signature
    /// check, available funds, etc...
    ///
    /// # Panics
    /// Any failure in the checks performed in processing the transaction will
    /// result in a panic. The contract expects the environment to roll back any
    /// change in state.
    fn spend_moonlight(&mut self, moonlight_tx: &MoonlightTransaction) {
        if moonlight_tx.chain_id() != self.chain_id() {
            panic!("The tx must target the correct chain");
        }

        // check the signature is valid and made by `sender`
        if !rusk_abi::verify_bls(
            moonlight_tx.signature_message(),
            *moonlight_tx.sender(),
            *moonlight_tx.signature(),
        ) {
            panic!("Invalid signature!");
        }

        // check `sender` has the funds necessary to suppress the total value
        // available in this transaction, and that the `nonce` is higher than
        // the currently held number. If these conditions are violated we panic
        // since the transaction is invalid - either because the account doesn't
        // have (enough) funds, or because they're possibly trying to reuse a
        // previously used signature (i.e. a replay attack).
        //
        // Afterwards, we simply deduct the total amount of the transaction from
        // the balance, increment the nonce, and rely on `refund` to be called
        // after a successful exit.
        //
        // TODO: this is expensive, maybe we should address the fact that
        //       `AccountPublicKey` doesn't `impl Ord` so we can just use it
        //       directly as a key in the `BTreeMap`
        let sender_bytes = moonlight_tx.sender().to_raw_bytes();

        // the total value carried by a transaction is the sum of the value, the
        // deposit, and gas_limit * gas_price.
        let total_value = moonlight_tx.value()
            + moonlight_tx.deposit()
            + moonlight_tx.gas_limit() * moonlight_tx.gas_price();

        match self.accounts.get_mut(&sender_bytes) {
            Some(account) => {
                if total_value > account.balance {
                    panic!("Account doesn't have enough funds");
                }

                // NOTE: exhausting the nonce is nearly impossible, since it
                //       requires performing more than 18 quintillion
                //       transactions. Since this number is so large, we also
                //       skip overflow checks.
                let incremented_nonce = account.nonce + 1;
                if moonlight_tx.nonce() < incremented_nonce {
                    panic!("Already used nonce");
                }
                if moonlight_tx.nonce() > incremented_nonce {
                    panic!("{PANIC_NONCE_NOT_READY}",);
                }

                account.balance -= total_value;
                account.nonce = moonlight_tx.nonce();
            }
            None => panic!("Account has no funds"),
        }

        // add the value to the receiver account
        if moonlight_tx.value() > 0 {
            // if the receiver has no entry, we simply instantiate a new one
            // with a zero nonce and balance.
            let receiver_bytes =
                moonlight_tx.receiver().map(|rcvr| rcvr.to_raw_bytes());
            let account = self
                .accounts
                .entry(receiver_bytes.unwrap_or(sender_bytes))
                .or_insert(EMPTY_ACCOUNT);
            account.balance += moonlight_tx.value();
        }
    }

    /// Refund the previously performed transaction, taking into account the
    /// given gas spent and a potential deposit that hasn't been picked up by
    /// the contract. The note produced will be refunded to the address present
    /// in the fee structure.
    ///
    /// This function guarantees that it will not panic.
    pub fn refund(&mut self, gas_spent: u64) {
        let ongoing = transitory::take_ongoing();

        // If there is a deposit still available on the call to this function,
        // we refund it to the called.
        let deposit = match ongoing.deposit {
            Deposit::Available { value, .. } => Some(value),
            _ => None,
        };

        let mut memo = Vec::new();
        if let Some(m) = ongoing.tx.memo() {
            memo = m.to_vec();
        }

        // the unspent gas is refunded to the refund-address specified in the
        // fee for both phoenix and moonlight transactions.
        //
        // any eventual deposit that failed to be "picked up" is also refunded
        // in the same way - in phoenix the same note is reused, in
        // moonlight the refund-address' balance gets increased.
        match ongoing.tx {
            Transaction::Phoenix(tx) => {
                let notes = ongoing.notes;

                let remainder_note =
                    tx.fee().gen_remainder_note(gas_spent, deposit);

                // if the refund-value is 0, we don't push the note onto the
                // tree and the refund-note will be None
                let refund_note = self.push_note_current_height(remainder_note);

                rusk_abi::emit(
                    PHOENIX_TOPIC,
                    PhoenixTransactionEvent {
                        nullifiers: tx.nullifiers().to_vec(),
                        notes,
                        memo,
                        gas_spent,
                        refund_note,
                    },
                );
            }
            Transaction::Moonlight(tx) => {
                let remaining_gas = tx.gas_limit() - gas_spent;
                let refund = remaining_gas * tx.gas_price()
                    + deposit.unwrap_or_default();

                let refund_account = self
                    .accounts
                    .entry(tx.refund_address().to_raw_bytes())
                    .or_insert(EMPTY_ACCOUNT);

                refund_account.balance += refund;

                let refund_info =
                    if refund > 0 && tx.refund_address() != tx.sender() {
                        Some((*tx.refund_address(), refund))
                    } else {
                        None
                    };

                rusk_abi::emit(
                    MOONLIGHT_TOPIC,
                    MoonlightTransactionEvent {
                        sender: *tx.sender(),
                        receiver: tx.receiver().copied(),
                        value: tx.value(),
                        memo,
                        gas_spent,
                        refund_info,
                    },
                );
            }
        }
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// height.
    pub fn leaves_from_height(&self, height: u64) {
        for leaf in self.tree.leaves(height) {
            rusk_abi::feed(leaf.clone());
        }
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// position.
    pub fn leaves_from_pos(&self, pos: u64) {
        self.sync(pos, 0)
    }

    /// Feeds the host with the leaves in the tree (up to `count_limit`
    /// occurrences), starting from the given `from` position.
    ///
    /// If `count_limit` is 0 there is no occurrences limit`
    pub fn sync(&self, from: u64, count_limit: u64) {
        let iter = self.tree.leaves_pos(from);

        if count_limit == 0 {
            for leaf in iter {
                rusk_abi::feed(leaf.clone());
            }
        } else {
            for leaf in iter.take(count_limit as usize) {
                rusk_abi::feed(leaf.clone());
            }
        }
    }

    pub fn sync_nullifiers(&self, from: u64, count_limit: u64) {
        let iter = self.nullifiers.iter().skip(from as usize);
        if count_limit == 0 {
            for n in iter {
                rusk_abi::feed(*n);
            }
        } else {
            for n in iter.take(count_limit as usize) {
                rusk_abi::feed(*n);
            }
        }
    }

    pub fn sync_contract_balances(&self, from: u64, count_limit: u64) {
        let iter = self.contract_balances.iter().skip(from as usize);

        if count_limit == 0 {
            for (contract, balance) in iter {
                rusk_abi::feed((*contract, *balance));
            }
        } else {
            for (contract, balance) in iter.take(count_limit as usize) {
                rusk_abi::feed((*contract, *balance));
            }
        }
    }

    pub fn sync_accounts(&self, from: u64, count_limit: u64) {
        let iter = self.accounts.iter().skip(from as usize);

        if count_limit == 0 {
            for (key, account) in iter {
                rusk_abi::feed((account.clone(), *key));
            }
        } else {
            for (key, account) in iter.take(count_limit as usize) {
                rusk_abi::feed((account.clone(), *key));
            }
        }
    }

    /// Update the root for of the tree.
    pub fn update_root(&mut self) {
        let root = self.tree.root();
        self.roots.push(root);
    }

    /// Get the root of the tree.
    pub fn root(&self) -> BlsScalar {
        self.tree.root()
    }

    /// Get the count of the notes in the tree.
    pub fn num_notes(&self) -> u64 {
        self.tree.leaves_len()
    }

    /// Get the opening
    pub fn opening(&self, pos: u64) -> Option<NoteOpening> {
        self.tree.opening(pos)
    }

    /// Takes some nullifiers and returns a vector containing the ones that
    /// already exists in the contract
    pub fn existing_nullifiers(
        &self,
        nullifiers: Vec<BlsScalar>,
    ) -> Vec<BlsScalar> {
        nullifiers
            .into_iter()
            .filter_map(|n| self.nullifiers.get(&n).map(|_| n))
            .collect()
    }

    pub fn account(&self, key: &AccountPublicKey) -> AccountData {
        let key_bytes = key.to_raw_bytes();
        self.accounts
            .get(&key_bytes)
            .cloned()
            .unwrap_or(EMPTY_ACCOUNT)
    }

    pub fn add_account_balance(&mut self, key: &AccountPublicKey, value: u64) {
        let key_bytes = key.to_raw_bytes();
        let account = self.accounts.entry(key_bytes).or_insert(EMPTY_ACCOUNT);
        account.balance = account.balance.saturating_add(value);
    }

    pub fn sub_account_balance(&mut self, key: &AccountPublicKey, value: u64) {
        let key_bytes = key.to_raw_bytes();
        if let Some(account) = self.accounts.get_mut(&key_bytes) {
            account.balance = account.balance.saturating_sub(value);
        }
    }

    /// Return the balance of a given contract.
    pub fn contract_balance(&self, contract_id: &ContractId) -> u64 {
        self.contract_balances
            .get(contract_id)
            .copied()
            .unwrap_or_default()
    }

    /// Add balance to the given contract
    pub fn add_contract_balance(&mut self, contract: ContractId, value: u64) {
        match self.contract_balances.entry(contract) {
            Entry::Vacant(ve) => {
                ve.insert(value);
            }
            Entry::Occupied(mut oe) => {
                let v = oe.get_mut();
                *v += value
            }
        }
    }

    pub(crate) fn sub_contract_balance(
        &mut self,
        address: &ContractId,
        value: u64,
    ) -> Result<(), Error> {
        match self.contract_balances.get_mut(address) {
            Some(balance) => {
                let (bal, underflow) = balance.overflowing_sub(value);

                if underflow {
                    Err(Error::NotEnoughBalance)
                } else {
                    *balance = bal;

                    Ok(())
                }
            }

            _ => Err(Error::NotEnoughBalance),
        }
    }

    fn root_exists(&self, root: &BlsScalar) -> bool {
        self.roots.contains(root)
    }

    pub fn push_note(&mut self, block_height: u64, note: Note) -> Option<Note> {
        self.tree.push(NoteLeaf { block_height, note })
    }

    fn push_note_current_height(&mut self, note: Note) -> Option<Note> {
        let block_height = rusk_abi::block_height();
        self.push_note(block_height, note)
    }

    pub fn chain_id(&self) -> u8 {
        rusk_abi::chain_id()
    }
}

fn verify_tx_proof(tx: &PhoenixTransaction) -> bool {
    // fetch the verifier data
    let num_inputs = tx.nullifiers().len();
    let vd = tx_circuit_verifier(num_inputs)
        .expect("No circuit available for given number of inputs!")
        .to_vec();

    // verify the proof
    rusk_abi::verify_plonk(vd, tx.proof().to_vec(), tx.public_inputs())
}

#[cfg(test)]
mod test_transfer {
    use super::*;

    #[test]
    fn find_existing_nullifiers() {
        let mut transfer = TransferState::new();

        let (zero, one, two, three, ten, eleven) = (
            BlsScalar::from(0),
            BlsScalar::from(1),
            BlsScalar::from(2),
            BlsScalar::from(3),
            BlsScalar::from(10),
            BlsScalar::from(11),
        );

        let existing = transfer
            .existing_nullifiers(vec![zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 0);

        for i in 1..10 {
            transfer.nullifiers.insert(BlsScalar::from(i));
        }

        let existing = transfer
            .existing_nullifiers(vec![zero, one, two, three, ten, eleven]);

        assert_eq!(existing.len(), 3);

        assert!(existing.contains(&one));
        assert!(existing.contains(&two));
        assert!(existing.contains(&three));
    }
}
