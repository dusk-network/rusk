export const transactionFragment = `
fragment TransactionInfo on SpentTransaction {
	blockHash,
	blockHeight,
	blockTimestamp,
  err,
	gasSpent,
	id,
  tx {
    ${import.meta.env.VITE_FEATURE_BLOB_HASHES === "true" ? "blobHashes," : ""}
    callData {
      contractId,
      data,
      fnName
    },
    gasLimit,
    gasPrice,
    id,
    isDeploy,
    memo,
    txType,
    json
  }
}
`;

const blockFragment = `
${transactionFragment}
fragment BlockInfo on Block {
  header {
    hash,
    gasLimit,
    height,
    prevBlockHash,
    seed,
    stateHash,
    timestamp,
    version
  },
  fees,
  gasSpent,
  reward,
  transactions {...TransactionInfo}
}
`;

/** @param {string} address */
export const getFullMoonlightAccountHistoryQuery = (address) => ({
  query: `
    query GetFullMoonlightHistory($address: String!) {
      fullMoonlightHistory(address: $address) {
        json
      }
    }
  `,
  variables: { address },
});

/** @param {number} height */
export const getBlockHashQueryInfo = (height) => ({
  query: `
    query($height: Float!) { block(height: $height) { header { hash } } }
  `,
  variables: { height },
});

/** @param {string} id */
export const getBlockQueryInfo = (id) => ({
  query: `
    ${blockFragment}
    query($id: String!) { block(hash: $id) {...BlockInfo} }
  `,
  variables: { id },
});

/** @param {string} id */
export const getBlockDetailsQueryInfo = (id) => ({
  query: "query($id: String!) { block(hash: $id) { header { json } } }",
  variables: { id },
});

/** @param {number} amount */
export const getBlocksQueryInfo = (amount) => ({
  query: `
    ${blockFragment}
    query($amount: Int!) { blocks(last: $amount) {...BlockInfo} }
  `,
  variables: { amount },
});

/** @param {number} amount */
export const getLatestChainQueryInfo = (amount) => ({
  query: `
    ${blockFragment}
    query($amount: Int!) {
      blocks(last: $amount) {...BlockInfo},
      transactions(last: $amount) {...TransactionInfo}
    }
  `,
  variables: { amount },
});

/** @param {string} id */
export const getMempoolTx = (id) => ({
  query: "query($id: String!) { mempoolTx(hash: $id) { isDeploy } }",
  variables: { id },
});

/** @param {number} amount */
export const getTransactionsQueryInfo = (amount) => ({
  query: `
    ${transactionFragment}
    query($amount: Int!) { transactions(last: $amount) {...TransactionInfo} }
  `,
  variables: { amount },
});

/** @param {string} id */
export const getTransactionQueryInfo = (id) => ({
  query: `
    ${transactionFragment}
    query($id: String!) {
      tx(hash: $id) {
        ...TransactionInfo
        tx {
          json
        }
      }
    }
  `,
  variables: { id },
});

/** @param {string} id */
export const searchByHashQueryInfo = (id) => ({
  query: `
    query($id: String!) {
      block(hash: $id) { header { hash } },
      tx(hash: $id) { id }
    }
  `,
  variables: { id },
});
