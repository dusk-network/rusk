import { validateMnemonic as bip39validateMnemonic } from "@scure/bip39";
import { wordlist } from "@scure/bip39/wordlists/english.js";

/** @param {string} mnemonic */
const validateMnemonic = (mnemonic) =>
  bip39validateMnemonic(mnemonic, wordlist);

export default validateMnemonic;
