import { skipIn } from "lamb";

import { unixTsToDate } from "$lib/dusk/date";

/** @type {(v: APITransaction) => Transaction} */
const transformTransaction = (v) => ({
  ...skipIn(v, ["__typename", "blocktimestamp", "blockts", "txtype"]),
  date: unixTsToDate(v.blockts),
  method: v.method ?? "",
});

export default transformTransaction;
