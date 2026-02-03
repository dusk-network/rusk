import { generateMnemonic as bip39GenerateMnemonic } from "@scure/bip39";
import { wordlist } from "@scure/bip39/wordlists/english.js";

const generateMnemonic = () => bip39GenerateMnemonic(wordlist, 128);

export default generateMnemonic;
