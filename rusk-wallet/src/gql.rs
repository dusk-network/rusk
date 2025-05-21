// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! The graphql endpoint can be queried with this helper struct.
//! The <node-url>/on/gaphql/query if queried with empty bytes returns the
//! graphql schema

use dusk_core::transfer::phoenix::StealthAddress;
use dusk_core::transfer::Transaction;
use dusk_core::transfer::{ConvertEvent, MoonlightTransactionEvent};
use serde::Deserialize;
use serde_json::Value;
use serde_with::hex::Hex;
use serde_with::{serde_as, DisplayFromStr};
use tokio::time::{sleep, Duration};

use crate::rues::HttpClient as RuesHttpClient;
use crate::{Address, Error};

/// GraphQL is a helper struct that aggregates all queries done
/// to the Dusk GraphQL database.
/// This helps avoid having helper structs and boilerplate code
/// mixed with the wallet logic.
#[derive(Clone)]
pub struct GraphQL {
    client: RuesHttpClient,
    status: fn(&str),
}

/// The `tx_for_block` returns a Vec<BlockTransaction> which contains
/// the dusk-core transaction, its id hash and gas spent
pub struct BlockTransaction {
    /// The dusk-core transaction struct obtained from GraphQL endpoint
    pub tx: Transaction,
    /// The hash of the transaction or the id of the transaction in string utf8
    pub id: String,
    /// Gas amount spent for the transaction
    pub gas_spent: u64,
}

#[derive(Deserialize)]
struct SpentTx {
    pub id: String,
    #[serde(default)]
    pub raw: String,
    pub err: Option<String>,
    #[serde(alias = "gasSpent", default)]
    pub gas_spent: u64,
}

#[derive(Deserialize)]
struct Block {
    pub transactions: Vec<SpentTx>,
}

#[derive(Deserialize)]
struct BlockResponse {
    pub block: Option<Block>,
}

// See `PhoenixTransactionEventSubset` for the reason for this struct
// and allowing dead code here.
#[serde_as]
#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
struct NoteAddress {
    stealth_address: StealthAddress,
}

// This struct is used instead of the one in `dusk_core::transfer`
// because of the order-dependent deserialization bug in
// the `dusk_core::transfer::phoenix::Note` https://github.com/dusk-network/phoenix/issues/274.
// Dead code is allowed to avoid catch-alls, so that the case in which an
// unexpected event is received, an appropriate error will be thrown.
#[serde_as]
#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
pub struct PhoenixTransactionEventSubset {
    /// Notes produced during the transaction.
    #[serde(rename(deserialize = "notes"))]
    note_addresses: Vec<NoteAddress>,
    /// The memo included in the transaction.
    #[serde_as(as = "Hex")]
    memo: Vec<u8>,
    /// Gas spent by the transaction.
    #[serde_as(as = "DisplayFromStr")]
    gas_spent: u64,
    /// Optional gas-refund note if the refund is positive.
    #[serde(rename(deserialize = "refund_note"))]
    refund_note_address: Option<NoteAddress>,
}

/// Deserialized block data in the full moonlight history.
#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum BlockData {
    /// For the moonlight transaction event.
    MoonlightTransactionEvent(MoonlightTransactionEvent),
    /// For the PhoenixTransactionEvent.
    /// In the case where a conversion is made from phoenix to
    /// moonlight, this appears.
    PhoenixTransactionEvent(PhoenixTransactionEventSubset),
    /// For the convert event.
    ConvertEvent(ConvertEvent),
}

#[derive(Deserialize, Debug)]
pub struct BlockEvents {
    pub data: BlockData,
}

#[derive(Deserialize, Debug)]
pub struct MoonlightHistory {
    pub block_height: u64,
    pub origin: String,
    pub events: Vec<BlockEvents>,
}

#[derive(Deserialize, Debug)]
pub struct MoonlightHistoryJson {
    pub json: Vec<MoonlightHistory>,
}

#[derive(Deserialize, Debug)]
pub struct FullMoonlightHistory {
    #[serde(rename(deserialize = "fullMoonlightHistory"))]
    pub full_moonlight_history: Option<MoonlightHistoryJson>,
}

#[derive(Deserialize)]
struct SpentTxResponse {
    pub tx: Option<SpentTx>,
}

/// Transaction status
#[derive(Debug)]
pub enum TxStatus {
    Ok,
    NotFound,
    Error(String),
}

impl GraphQL {
    /// Create a new GraphQL wallet client
    ///
    /// # Errors
    /// This method errors if a TLS backend cannot be initialized, or the
    /// resolver cannot load the system configuration.
    pub fn new<S: Into<String>>(
        url: S,
        status: fn(&str),
    ) -> Result<Self, Error> {
        Ok(Self {
            client: RuesHttpClient::new(url)?,
            status,
        })
    }

    /// Wait for a transaction to be confirmed (included in a block)
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format or encoded correctly.
    pub async fn wait_for(&self, tx_id: &str) -> anyhow::Result<()> {
        loop {
            let status = self.tx_status(tx_id).await?;

            match status {
                TxStatus::Ok => break,
                TxStatus::Error(err) => return Err(Error::Transaction(err))?,
                TxStatus::NotFound => {
                    (self.status)(
                        "Waiting for tx to be included into a block...",
                    );
                    sleep(Duration::from_millis(1000)).await;
                }
            }
        }
        Ok(())
    }

    /// Obtain transaction status
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// if the response body is not in JSON format or encoded correctly or if
    /// the transaction couldn't be found.
    async fn tx_status(&self, tx_id: &str) -> Result<TxStatus, Error> {
        let query =
            "query { tx(hash: \"####\") { id, err }}".replace("####", tx_id);
        let response = self.query(&query).await?;
        let response = serde_json::from_slice::<SpentTxResponse>(&response)?.tx;

        match response {
            Some(SpentTx { err: Some(err), .. }) => Ok(TxStatus::Error(err)),
            Some(_) => Ok(TxStatus::Ok),
            None => Ok(TxStatus::NotFound),
        }
    }

    /// Obtain transactions inside a block
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format or encoded correctly.
    pub async fn txs_for_block(
        &self,
        block_height: u64,
    ) -> Result<Vec<BlockTransaction>, Error> {
        let query = "query { block(height: ####) { transactions {id, raw, gasSpent, err}}}"
            .replace("####", block_height.to_string().as_str());

        let response = self.query(&query).await?;
        let response =
            serde_json::from_slice::<BlockResponse>(&response)?.block;
        let block = response.ok_or(GraphQLError::BlockInfo)?;
        let mut ret = vec![];

        for spent_tx in block.transactions {
            let tx_raw = hex::decode(&spent_tx.raw)
                .map_err(|_| GraphQLError::TxStatus)?;
            let ph_tx = Transaction::from_slice(&tx_raw)
                .map_err(|_| GraphQLError::BytesError)?;
            ret.push(BlockTransaction {
                tx: ph_tx,
                id: spent_tx.id,
                gas_spent: spent_tx.gas_spent,
            });
        }

        Ok(ret)
    }

    /// Sends an empty body to url to check if its available
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query.
    pub async fn check_connection(&self) -> Result<(), Error> {
        self.query("").await.map(|_| ())
    }

    /// Query the archival node for moonlight transactions given the
    /// `BlsPublicKey`
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format or encoded correctly.
    pub async fn moonlight_history(
        &self,
        public_address: Address,
    ) -> Result<FullMoonlightHistory, Error> {
        let query = format!(
            r#"query {{ fullMoonlightHistory(address: "{public_address}") {{ json }} }}"#
        );

        let response = self
            .query(&query)
            .await
            .map_err(|err| Error::ArchiveJsonError(err.to_string()))?;

        let response =
            serde_json::from_slice::<FullMoonlightHistory>(&response)
                .map_err(|err| Error::ArchiveJsonError(err.to_string()))?;

        Ok(response)
    }

    /// Query the archival node for moonlight transactions of `public_address`
    /// in the given `block`.
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format or encoded correctly.
    pub async fn moonlight_history_at_block(
        &self,
        public_address: &Address,
        block: u64,
    ) -> Result<FullMoonlightHistory, Error> {
        let query = format!(
            r#"query {{ fullMoonlightHistory(address: "{public_address}", fromBlock: {block}, toBlock: {block}) {{ json }} }}"#
        );

        let response = self
            .query(&query)
            .await
            .map_err(|err| Error::ArchiveJsonError(err.to_string()))?;

        let response =
            serde_json::from_slice::<FullMoonlightHistory>(&response)
                .map_err(|err| Error::ArchiveJsonError(err.to_string()))?;

        Ok(response)
    }

    /// Fetch the spent transaction given moonlight tx hash
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format or encoded correctly.
    pub async fn moonlight_tx(
        &self,
        origin: &str,
    ) -> Result<Transaction, Error> {
        let query =
            format!(r#"query {{ tx(hash: "{origin}") {{ tx {{ raw }} }} }}"#);

        let response = self.query(&query).await?;
        let json: Value = serde_json::from_slice(&response)?;

        let tx = json
            .get("tx")
            .and_then(|val| val.get("tx").and_then(|val| val.get("raw")))
            .and_then(|val| val.as_str());

        if let Some(tx) = tx {
            let hex = hex::decode(tx).map_err(|_| GraphQLError::TxStatus)?;
            let tx: Transaction = Transaction::from_slice(&hex)?;
            Ok(tx)
        } else {
            Err(Error::GraphQLError(GraphQLError::TxStatus))
        }
    }
}

/// Errors generated from GraphQL
#[derive(Debug, thiserror::Error)]
pub enum GraphQLError {
    /// Generic errors
    #[error("Error fetching data from the node: {0}")]
    Generic(serde_json::Error),
    /// Failed to fetch transaction status
    #[error("Failed to obtain transaction status")]
    TxStatus,
    #[error("Failed to obtain block info")]
    /// Failed to obtain block info
    BlockInfo,
    /// Bytes decoding errors
    #[error("A deserialization error occurred")]
    BytesError,
}

impl From<serde_json::Error> for GraphQLError {
    fn from(e: serde_json::Error) -> Self {
        Self::Generic(e)
    }
}

impl GraphQL {
    /// Call the graphql endpoint of a node
    ///
    /// # Errors
    /// This method errors if there was an error while sending the query,
    /// or if the response body is not in JSON format.
    pub async fn query(&self, query: &str) -> Result<Vec<u8>, Error> {
        self.client
            .call("graphql", None, "query", query.as_bytes())
            .await
    }
}

#[ignore = "Leave it here just for manual tests"]
#[tokio::test]
async fn test() -> Result<(), Error> {
    let gql = GraphQL {
        status: |s| {
            println!("{s}");
        },
        client: RuesHttpClient::new(
            "http://testnet.nodes.dusk.network:9500/graphql",
        )?,
    };
    let _ = gql
        .tx_status(
            "dbc5a2c949516ecfb418406909d195c3cc267b46bd966a3ca9d66d2e13c47003",
        )
        .await?;
    let block_txs = gql.txs_for_block(90).await?;
    block_txs.into_iter().for_each(|tx_block| {
        let tx = tx_block.tx;
        let chain_txid = tx_block.id;
        let hash = tx.hash();
        let tx_id = hex::encode(hash.to_bytes());
        assert_eq!(chain_txid, tx_id);
        println!("txid: {tx_id}");
    });
    Ok(())
}

#[tokio::test]
async fn deser() -> Result<(), Box<dyn std::error::Error>> {
    let block_not_found = r#"{"block":null}"#;
    serde_json::from_str::<BlockResponse>(block_not_found).unwrap();

    let block_without_tx = r#"{"block":{"transactions":[]}}"#;
    serde_json::from_str::<BlockResponse>(block_without_tx).unwrap();

    let block_with_tx = r#"{"block":{"transactions":[{"id":"88e6804989cc2f3fd5bf94dcd39a4e7b7da9a1114d9b8bf4e0515264bc81c50f"}]}}"#;
    serde_json::from_str::<BlockResponse>(block_with_tx).unwrap();

    let empty_full_moonlight_history = r#"{"fullMoonlightHistory":null}"#;
    serde_json::from_str::<FullMoonlightHistory>(empty_full_moonlight_history)
        .unwrap();

    let full_moonlight_history =
        include_str!("./gql/tests/assets/full_moonlight_history.json");
    serde_json::from_str::<FullMoonlightHistory>(full_moonlight_history)
        .unwrap();

    Ok(())
}
