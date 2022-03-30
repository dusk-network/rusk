// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use gql_client::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

use super::error::GraphQLError;

/// GraphQL is a helper struct that aggregates all queries done
/// to the Dusk GraphQL database.
/// This helps avoid having helper structs and boilerplate code
/// mixed with the wallet logic
#[derive(Clone, Debug)]
pub struct GraphQL {
    url: String,
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
    pub fn new<S>(url: S) -> Self
    where
        S: Into<String>,
    {
        Self { url: url.into() }
    }

    /// Obtains current block height from GraphQL endpoint
    pub fn current_block_height(&self) -> Result<u64, GraphQLError> {
        // graphql connection
        let client = Client::new(&self.url);

        // helper structs to deserialize response
        #[derive(Deserialize)]
        struct Height {
            pub height: u64,
        }
        #[derive(Deserialize)]
        struct Header {
            pub header: Height,
        }
        #[derive(Deserialize)]
        struct Blocks {
            pub blocks: Vec<Header>,
        }

        // query the db
        let query = "{blocks(last:1){header{height}}}";
        let res = block_in_place(move || {
            Handle::current()
                .block_on(async move { client.query::<Blocks>(query).await })
        })?;

        // collect response
        if let Some(r) = res {
            if !r.blocks.is_empty() {
                let h = r.blocks[0].header.height;
                return Ok(h);
            }
        }
        Err(GraphQLError::BlockHeight)
    }

    /// Obtain transaction status
    pub fn tx_status(&self, tx_id: &str) -> Result<TxStatus, GraphQLError> {
        // graphql connection
        let client = Client::new(&self.url);

        // helper structs to deserialize response
        #[derive(Deserialize)]
        struct Tx {
            pub txerror: String,
        }
        #[derive(Deserialize)]
        struct Transactions {
            pub transactions: Vec<Tx>,
        }

        let query =
            "{transactions(txid:\"####\"){ txerror }}".replace("####", tx_id);

        let response = block_in_place(move || {
            Handle::current().block_on(async move {
                client.query::<Transactions>(&query).await
            })
        });

        // we're interested in different types of errors
        if response.is_err() {
            let err = response.err().unwrap();
            return match err.json() {
                Some(json) => {
                    // we stringify the json and use String.contains()
                    // because GraphQLErrorMessage fields are private
                    let json_str = format!("{:?}", json[0]);
                    if json_str.contains("database: transaction not found") {
                        Ok(TxStatus::NotFound)
                    } else {
                        Err(GraphQLError::Generic(err))
                    }
                }
                None => Err(GraphQLError::Generic(err)),
            };
        }

        // fetch and parse the response data
        let data = response.expect("GQL response failed");
        match data {
            Some(txs) => {
                if txs.transactions.is_empty() {
                    Ok(TxStatus::NotFound)
                } else {
                    let tx = &txs.transactions[0];
                    if tx.txerror.is_empty() {
                        Ok(TxStatus::Ok)
                    } else {
                        let err_str = tx.txerror.as_str();
                        let tx_err = serde_json::from_str::<Value>(err_str);
                        match tx_err {
                            Ok(data) => match data["data"].as_str() {
                                Some(msg) => {
                                    Ok(TxStatus::Error(msg.to_string()))
                                }
                                None => {
                                    Ok(TxStatus::Error(err_str.to_string()))
                                }
                            },
                            Err(err) => Ok(TxStatus::Error(err.to_string())),
                        }
                    }
                }
            }
            None => Err(GraphQLError::TxStatus),
        }
    }
}
