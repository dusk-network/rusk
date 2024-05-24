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
