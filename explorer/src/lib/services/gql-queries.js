const transactionFragment = `
fragment TransactionInfo on SpentTransaction {
	blockHash,
	blockHeight,
	blockTimestamp,
  err,
	gasSpent,
	id,
  tx {
    callData {
      contractId,
      data,
      fnName
    },
    gasLimit,
    gasPrice,
    id,
    isDeploy,
    memo
    txType
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
export const getTransactionQueryInfo = (id) => ({
  query: `
    ${transactionFragment}
    query($id: String!) { tx(hash: $id) {...TransactionInfo} }
  `,
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
export const getTransactionDetailsQueryInfo = (id) => ({
  query: "query($id: String!) { tx(hash: $id) { raw } }",
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
