import { getKey, sortWith, sorterDesc } from "lamb";

/** @type {(transactions: Transaction[]) => Transaction[]} */
const sortByHeightDesc = sortWith([sorterDesc(getKey("block_height"))]);

export default sortByHeightDesc;
