/**
 * Constructs a node URL based on the current subdomain
 *
 * @param {string} path
 * @returns {URL} nodeUrl
 */
function makeNodeUrl(path = "") {
  if (path !== "" && !path.startsWith("/")) {
    throw new Error("A path must start with a '/'.");
  }

  const subDomains = window.location.hostname.split(".");
  const hostedNodeDomain = subDomains.slice(-2).join(".");
  const nodeBaseUrl = import.meta.env.VITE_NODE_URL
    ? import.meta.env.VITE_NODE_URL.replace(/\/+$/, "")
    : "";
  const nodeBasePath = import.meta.env.VITE_RUSK_PATH
    ? import.meta.env.VITE_RUSK_PATH.replace(/^\/?/, "/")
    : "";

  /**
   * @param {string} base
   * @returns {URL}
   */
  const buildHostedNodeUrl = (base) =>
    new URL(
      `${window.location.protocol}${base}${hostedNodeDomain}${nodeBasePath}${path}`
    );

  let nodeUrl;

  switch (`${subDomains[0]}.${subDomains[1]}.${subDomains[2]}`) {
    case "apps.dusk.network":
    case "apps.staging.dusk":
      nodeUrl = buildHostedNodeUrl("nodes.");
      break;
    case "apps.devnet.dusk":
    case "apps.staging.devnet":
      nodeUrl = buildHostedNodeUrl("devnet.nodes.");
      break;
    case "apps.testnet.dusk":
    case "apps.staging.testnet":
      nodeUrl = buildHostedNodeUrl("testnet.nodes.");
      break;
    default:
      nodeUrl = new URL(
        `${nodeBaseUrl}${nodeBasePath}${path}`,
        import.meta.url
      );
      break;
  }

  return nodeUrl;
}

export default makeNodeUrl;
