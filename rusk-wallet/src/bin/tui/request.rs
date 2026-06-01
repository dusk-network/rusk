// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use rusk_wallet::Address;
use rusk_wallet::currency::Dusk;

use crate::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TuiCommand {
    Transfer {
        sender: Box<Address>,
        rcvr: Box<Address>,
        amt: Dusk,
        gas_limit: u64,
        gas_price: u64,
        memo: Option<String>,
    },
    Stake {
        address: Box<Address>,
        owner: Box<Address>,
        amt: Dusk,
        gas_limit: u64,
        gas_price: u64,
    },
    Unstake {
        address: Box<Address>,
        gas_limit: u64,
        gas_price: u64,
    },
    ClaimRewards {
        address: Box<Address>,
        reward: Option<Dusk>,
        gas_limit: u64,
        gas_price: u64,
    },
    Shield {
        profile_idx: u8,
        amt: Dusk,
        gas_limit: u64,
        gas_price: u64,
    },
    Unshield {
        profile_idx: u8,
        amt: Dusk,
        gas_limit: u64,
        gas_price: u64,
    },
    ContractDeploy {
        address: Box<Address>,
        code: PathBuf,
        init_args: Vec<u8>,
        deploy_nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    },
    ContractCall {
        address: Box<Address>,
        contract_id: Vec<u8>,
        fn_name: String,
        fn_args: Vec<u8>,
        gas_limit: u64,
        gas_price: u64,
        deposit: Dusk,
    },
    Export {
        profile_idx: u8,
        dir: PathBuf,
    },
}

impl TuiCommand {
    pub(crate) fn into_command(self) -> Command {
        match self {
            Self::Transfer {
                sender,
                rcvr,
                amt,
                gas_limit,
                gas_price,
                memo,
            } => Command::Transfer {
                sender: Some(*sender),
                rcvr: *rcvr,
                amt,
                gas_limit,
                gas_price,
                memo,
            },
            Self::Stake {
                address,
                owner,
                amt,
                gas_limit,
                gas_price,
            } => Command::Stake {
                address: Some(*address),
                owner: Some(*owner),
                amt,
                gas_limit,
                gas_price,
            },
            Self::Unstake {
                address,
                gas_limit,
                gas_price,
            } => Command::Unstake {
                address: Some(*address),
                gas_limit,
                gas_price,
            },
            Self::ClaimRewards {
                address,
                reward,
                gas_limit,
                gas_price,
            } => Command::ClaimRewards {
                address: Some(*address),
                reward,
                gas_limit,
                gas_price,
            },
            Self::Shield {
                profile_idx,
                amt,
                gas_limit,
                gas_price,
            } => Command::Shield {
                profile_idx: Some(profile_idx),
                amt,
                gas_limit,
                gas_price,
            },
            Self::Unshield {
                profile_idx,
                amt,
                gas_limit,
                gas_price,
            } => Command::Unshield {
                profile_idx: Some(profile_idx),
                amt,
                gas_limit,
                gas_price,
            },
            Self::ContractDeploy {
                address,
                code,
                init_args,
                deploy_nonce,
                gas_limit,
                gas_price,
            } => Command::ContractDeploy {
                address: Some(*address),
                code,
                init_args,
                deploy_nonce,
                gas_limit,
                gas_price,
            },
            Self::ContractCall {
                address,
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
                deposit,
            } => Command::ContractCall {
                address: Some(*address),
                contract_id,
                fn_name,
                fn_args,
                gas_limit,
                gas_price,
                deposit,
            },
            Self::Export { profile_idx, dir } => Command::Export {
                profile_idx: Some(profile_idx),
                dir,
                name: None,
                export_pwd: None,
            },
        }
    }

    pub(crate) fn refreshes_stake_info(&self) -> bool {
        matches!(
            self,
            Self::Stake { .. }
                | Self::Unstake { .. }
                | Self::ClaimRewards { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use rusk_wallet::Profile;
    use wallet_core::Seed;
    use wallet_core::keys::{derive_bls_pk, derive_phoenix_pk};

    use super::*;

    fn test_profile_at(index: u8) -> Profile {
        let seed: Seed = [7u8; 64];
        Profile {
            shielded_addr: derive_phoenix_pk(&seed, index),
            public_addr: derive_bls_pk(&seed, index),
        }
    }

    fn public_address(index: u8) -> Address {
        Address::Public(test_profile_at(index).public_addr)
    }

    fn shielded_address(index: u8) -> Address {
        Address::Shielded(test_profile_at(index).shielded_addr)
    }

    fn boxed(address: Address) -> Box<Address> {
        Box::new(address)
    }

    #[test]
    fn into_command_preserves_swappable_address_fields() {
        let sender = public_address(0);
        let rcvr = shielded_address(1);
        let command = TuiCommand::Transfer {
            sender: boxed(sender.clone()),
            rcvr: boxed(rcvr.clone()),
            amt: Dusk::from(1),
            gas_limit: 2,
            gas_price: 3,
            memo: None,
        }
        .into_command();
        let Command::Transfer {
            sender: Some(actual_sender),
            rcvr: actual_rcvr,
            ..
        } = command
        else {
            panic!("expected transfer command");
        };
        assert_eq!(actual_sender, sender);
        assert_eq!(actual_rcvr, rcvr);

        let stake_address = shielded_address(2);
        let stake_owner = public_address(3);
        let command = TuiCommand::Stake {
            address: boxed(stake_address.clone()),
            owner: boxed(stake_owner.clone()),
            amt: Dusk::from(4),
            gas_limit: 5,
            gas_price: 6,
        }
        .into_command();
        let Command::Stake {
            address: Some(actual_address),
            owner: Some(actual_owner),
            ..
        } = command
        else {
            panic!("expected stake command");
        };
        assert_eq!(actual_address, stake_address);
        assert_eq!(actual_owner, stake_owner);
    }

    #[test]
    fn refreshes_stake_info_for_stake_mutations() {
        let address = public_address(0);

        assert!(
            TuiCommand::Stake {
                address: boxed(address.clone()),
                owner: boxed(public_address(1)),
                amt: Dusk::from(1),
                gas_limit: 1,
                gas_price: 1,
            }
            .refreshes_stake_info()
        );
        assert!(
            TuiCommand::Unstake {
                address: boxed(address.clone()),
                gas_limit: 1,
                gas_price: 1,
            }
            .refreshes_stake_info()
        );
        assert!(
            TuiCommand::ClaimRewards {
                address: boxed(address.clone()),
                reward: None,
                gas_limit: 1,
                gas_price: 1,
            }
            .refreshes_stake_info()
        );
        assert!(
            !TuiCommand::Transfer {
                sender: boxed(address),
                rcvr: boxed(shielded_address(1)),
                amt: Dusk::from(1),
                gas_limit: 1,
                gas_price: 1,
                memo: None,
            }
            .refreshes_stake_info()
        );
    }
}
