import { always, compose, getKey, head } from "lamb";

import { walletStore } from "$lib/stores";

import sortByHeightDesc from "./sortByHeightDesc";

const getFirstHash = compose(getKey("id"), head);

/** @type {() => Promise<string>} */
const getLastTransactionHash = () =>
  walletStore
    .getTransactionsHistory()
    .then(sortByHeightDesc)
    .then(getFirstHash)
    .catch(always(""));

export default getLastTransactionHash;
