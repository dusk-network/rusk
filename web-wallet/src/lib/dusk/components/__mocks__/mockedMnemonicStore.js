import { mockDerivedStore, mockReadableStore } from "$lib/dusk/test-helpers";

/** @type {Array<string>} */
const enteredSeed = [];
const seed = [
  "serendipity",
  "quixotic",
  "mellifluous",
  "resplendent",
  "nebulous",
  "jubilant",
  "capricious",
  "pernicious",
  "ephemeral",
  "ineffable",
  "mellifluous",
  "effervescent",
];

/** @param {Array<string>} initialSeed */
const deriveFn = (initialSeed) => {
  return initialSeed;
};

export const mockedMnemonicPhrase = mockReadableStore(seed);
export const mockedEnteredMnemonicPhrase = mockReadableStore(enteredSeed);
export const mockedShuffledMnemonicPhrase = mockDerivedStore(seed, deriveFn);
