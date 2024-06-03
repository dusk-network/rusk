import { compose, flatMapWith, ownValues, reduce } from "lamb";

/** @type {(v: GQLSearchResult[]) => SearchResult[]} */
const transformSearchResult = compose(
  (entries) =>
    reduce(
      entries,
      (result, entry) => {
        if (entry) {
          result.push(
            entry.header?.hash
              ? {
                  id: entry.header.hash,
                  type: "block",
                }
              : {
                  id: entry.id,
                  type: "transaction",
                }
          );
        }

        return result;
      },
      /** @type {SearchResult[]} */ ([])
    ),
  flatMapWith(ownValues)
);

export default transformSearchResult;
