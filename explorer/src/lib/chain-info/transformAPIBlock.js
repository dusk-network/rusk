import { mapWith, pipe, skip, updateIn } from "lamb";

import { unixTsToDate } from "$lib/dusk/date";

import { transformAPITransaction } from ".";

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
const transformAPIBlockHeader = pipe([
  mergeWithDefaults,
  addHeaderDate,
  skip(["__typename", "timestamp", "ts", "version"]),
]);

/** @type {(v: APIBlock) => Block} */
const transformAPIBlock = ({ header, transactions }) => ({
  header: transformAPIBlockHeader(header),
  transactions: updateIn(
    transactions,
    "data",
    mapWith(transformAPITransaction)
  ),
});

export default transformAPIBlock;
