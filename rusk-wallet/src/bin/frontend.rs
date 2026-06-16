// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use dusk_core::BlsScalar;
use dusk_core::abi::ContractId;
use rusk_wallet::currency::Dusk;
use url::Url;
use wallet_core::BalanceInfo;

/// Frontend-facing balance state shared by the CLI and TUI.
#[derive(Debug, Clone, Default)]
pub(crate) struct BalanceView {
    pub phoenix: Option<BalanceInfo>,
    pub moonlight: Option<Dusk>,
}

impl BalanceView {
    pub(crate) fn shielded(balance: BalanceInfo) -> Self {
        Self {
            phoenix: Some(balance),
            moonlight: None,
        }
    }

    pub(crate) fn public(balance: Dusk) -> Self {
        Self {
            phoenix: None,
            moonlight: Some(balance),
        }
    }

    pub(crate) fn merge(&mut self, update: Self) {
        if let Some(balance) = update.phoenix {
            self.phoenix = Some(balance);
        }
        if let Some(balance) = update.moonlight {
            self.moonlight = Some(balance);
        }
    }

    pub(crate) fn shielded_total(&self) -> Option<Dusk> {
        self.phoenix
            .as_ref()
            .map(|balance| Dusk::from(balance.value))
    }

    pub(crate) fn shielded_spendable(&self) -> Option<Dusk> {
        self.phoenix
            .as_ref()
            .map(|balance| Dusk::from(balance.spendable))
    }
}

/// Frontend-facing operation result shared by the CLI and TUI.
#[derive(Debug)]
pub(crate) enum OperationResult {
    Tx(BlsScalar),
    DeployTx {
        hash: BlsScalar,
        contract_id: ContractId,
    },
    ExportedKeys {
        pub_key: PathBuf,
        key_pair: PathBuf,
    },
    Error {
        message: String,
    },
}

impl OperationResult {
    pub(crate) fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    pub(crate) fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    pub(crate) fn has_explorer_target(&self) -> bool {
        matches!(self, Self::Tx(_) | Self::DeployTx { .. })
    }

    pub(crate) fn tx_hash_hex(&self) -> Option<String> {
        match self {
            Self::Tx(hash) | Self::DeployTx { hash, .. } => {
                Some(hex::encode(hash.to_bytes()))
            }
            Self::ExportedKeys { .. } | Self::Error { .. } => None,
        }
    }

    pub(crate) fn contract_id_hex(&self) -> Option<String> {
        match self {
            Self::DeployTx { contract_id, .. } => {
                Some(hex::encode(contract_id.as_bytes()))
            }
            Self::Tx(_) | Self::ExportedKeys { .. } | Self::Error { .. } => {
                None
            }
        }
    }

    pub(crate) fn explorer_url(
        &self,
        explorer_base: Option<&Url>,
    ) -> Option<String> {
        explorer_base.and_then(|base| {
            self.tx_hash_hex().map(|hash| format!("{base}{hash}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use dusk_core::BlsScalar;

    use super::OperationResult;

    #[test]
    fn explorer_target_is_limited_to_transaction_results() {
        assert!(
            OperationResult::Tx(BlsScalar::from(1u64)).has_explorer_target()
        );
        assert!(
            !OperationResult::ExportedKeys {
                pub_key: PathBuf::from("public.key"),
                key_pair: PathBuf::from("keypair.key"),
            }
            .has_explorer_target()
        );
        assert!(!OperationResult::error("failed").has_explorer_target());
    }
}
