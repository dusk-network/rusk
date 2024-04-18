import { mapWith, pipe, skip, updateIn } from "lamb";

import { unixTsToDate } from "$lib/dusk/date";

import { transformTransaction } from ".";

/** @type {(v: APIBlockHeader) => Required<APIBlockHeader>} */
const mergeWithDefaults = (v) => ({
  nextblockhash: "",
  prevblockhash: "",
  statehash: "",
  ...v,
});

/** @type {(header: Required<APIBlockHeader>) => BlockHeader} */
const addHeaderDate = (header) => ({
  ...header,
  date: unixTsToDate(header.ts),
});

/** @type {(v: APIBlockHeader) => BlockHeader} */
const transformBlockHeader = pipe([
  mergeWithDefaults,
  addHeaderDate,
  skip(["__typename", "timestamp", "ts"]),
]);

/** @type {(v: APIBlock) => Block} */
const transformBlock = ({ header, transactions }) => ({
  header: transformBlockHeader(header),
  transactions: updateIn(transactions, "data", mapWith(transformTransaction)),
});

export default transformBlock;
