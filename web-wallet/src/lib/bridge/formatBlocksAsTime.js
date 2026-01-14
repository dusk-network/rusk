import { createNumberFormatter } from "$lib/dusk/number";

// Dusk ~= 10 seconds/block
const BLOCKS_PER_MIN = 6n;
const BLOCKS_PER_HOUR = 360n;
const BLOCKS_PER_DAY = 8640n;

/**
 * Approximate time from a Dusk block count.
 *
 * @param {bigint | number} blocks
 * @param {string} locale
 * @returns {string}
 */
export function formatBlocksAsTime(blocks, locale) {
  const b = typeof blocks === "bigint" ? blocks : BigInt(blocks);
  const abs = b < 0n ? -b : b;

  const fmt = createNumberFormatter(locale);

  if (abs < BLOCKS_PER_MIN) return "≈<1 min";

  if (abs < BLOCKS_PER_HOUR) {
    const mins = (abs + BLOCKS_PER_MIN / 2n) / BLOCKS_PER_MIN;
    return `≈${fmt(mins)} min`;
  }

  if (abs < BLOCKS_PER_DAY) {
    const hours = (abs + BLOCKS_PER_HOUR / 2n) / BLOCKS_PER_HOUR;
    return `≈${fmt(hours)} h`;
  }

  const days = (abs + BLOCKS_PER_DAY / 2n) / BLOCKS_PER_DAY;
  return `≈${fmt(days)} day${days === 1n ? "" : "s"}`;
}
