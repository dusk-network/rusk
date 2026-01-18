/**
 * Recipient address blocklist.
 *
 * IMPORTANT:
 * - This is a client-side safety mechanism.
 * - It cannot prevent transfers from other wallets or direct chain interaction.
 *
 * @typedef {{
 *   address: string;
 *   reason: string;
 * }} BlocklistedAddress
 */

/**
 * Hardcoded/default blocklist.
 *
 * Add known compromised recipients here.
 *
 * @type {BlocklistedAddress[]}
 */
export const DEFAULT_BLOCKLISTED_ADDRESSES = [
  {
    address:
      "mFqH6RVxCoWQfjQ23H9YVr8JhQ697M5DD4ob76kcuoEYqqYA6H9cxHrxjvnZ6z4PQKsBd3PBpRYLN9M3FgkQQVywREzkzgeme4ersJgLaxYaQzZSAzkd1QBJ4ByTe9NrhXp",
    reason:
      "Transfer restricted: this recipient address has been reported as compromised.",
  },
  {
    address:
      "25kt7vYP5JNJ1vborhco4dJBFfdHsFrS8oBsPZ6Q2wWmE6FYwahaE1heg8HSgc8C6KwHxpm19WxrgCuCtBDBfTJD6WNLi8jMqC2nkz6dEE3oepoMLsEYQnopTi5oPzfr7wGk",
    reason:
      "Transfer restricted: this recipient address is associated with reported fraud/scam activity.",
  },
];

/** @type {Map<string, BlocklistedAddress>} */
const BLOCKLIST_MAP = (() => {
  const map = new Map();

  for (const entry of DEFAULT_BLOCKLISTED_ADDRESSES) {
    const key = entry.address.trim();
    if (!key) continue;

    // Prefer the first entry.
    if (!map.has(key)) {
      map.set(key, entry);
    }
  }

  return map;
})();

/**
 * @returns {Map<string, BlocklistedAddress>}
 */
export function getRecipientBlocklist() {
  return BLOCKLIST_MAP;
}

/**
 * @param {string} address
 * @returns {BlocklistedAddress | undefined}
 */
export function getBlocklistedRecipient(address) {
  if (typeof address !== "string") {
    return undefined;
  }

  const key = address.trim();

  if (!key) {
    return undefined;
  }

  return BLOCKLIST_MAP.get(key);
}

/**
 * @param {string} address
 * @returns {boolean}
 */
export function isRecipientBlocklisted(address) {
  return Boolean(getBlocklistedRecipient(address));
}
