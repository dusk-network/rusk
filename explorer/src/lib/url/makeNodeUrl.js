/**
 * Constructs a node URL based on the current subdomain
 *
 * @param {string} path
 * @returns {URL} nodeUrl
 */
function makeNodeUrl(path = "") {
  if (path && !path.startsWith("/")) {
    throw new Error("A path must start with a '/'.");
  }

  const subDomains = window.location.hostname.split(".");
  const hostedNodeDomain = subDomains.slice(1).join(".");
  const nodeBaseUrl = import.meta.env.VITE_NODE_URL || "";
  const nodeBasePath = import.meta.env.VITE_RUSK_PATH || "";

  /**
   * @param {string} base
   * @returns {URL}
   */
  const buildHostedNodeUrl = (base) =>
    new URL(
      `${window.location.protocol}${base}${hostedNodeDomain}${nodeBasePath}${path}`
    );

  let node;

  switch (subDomains[0]) {
    case "apps": // mainnet
      node = buildHostedNodeUrl("nodes.");
      break;
    case "devnet":
      node = buildHostedNodeUrl("devnet.nodes.");
      break;
    case "testnet":
      node = buildHostedNodeUrl("testnet.nodes.");
      break;
    default:
      node = new URL(`${nodeBaseUrl}${nodeBasePath}${path}`, import.meta.url);
      break;
  }

  return node;
}

export default makeNodeUrl;
