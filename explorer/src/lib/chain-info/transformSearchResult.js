/**
 * Transforms a GQL search result into a standardized search result.
 * @param {GQLSearchResult} entry - The GQL search result to transform
 * @returns {SearchResult | null} The transformed search result
 */
const transformSearchResult = (entry) => {
  if (entry.block) {
    return {
      id: entry.block.header.hash,
      type: "block",
    };
  } else if (entry.tx) {
    return {
      id: entry.tx.id,
      type: "transaction",
    };
  } else if (entry.account) {
    return {
      id: entry.account.id,
      type: "account",
    };
  }

  return null;
};

export default transformSearchResult;
