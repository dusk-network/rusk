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
    id
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

/** @param {string} id */
export const getTransactionQueryInfo = (id) => ({
  query: `
    ${transactionFragment}
    query($id: String!) { tx(hash: $id) {...TransactionInfo} }
  `,
  variables: { id },
});

/** @param {string} id */
export const getTransactionDetailsQueryInfo = (id) => ({
  query: "query($id: String!) { tx(hash: $id) { raw } }",
  variables: { id },
});