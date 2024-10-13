/**
 * Constructs a node URL based on the current subdomain
 *
 * @returns {URL} nodeUrl
 */
function makeNodeUrl(path = "") {
  const domains = window.location.hostname.split(".");

  let node;

  switch (domains[0]) {
    case "apps": // mainnet
      node = new URL(
        `${window.location.protocol}nodes.${window.location.host}${path}`
      );
      break;
    case "devnet":
      node = new URL(
        `${window.location.protocol}devnet.nodes.${window.location.host}${path}`
      );
      break;
    case "testnet":
      node = new URL(
        `${window.location.protocol}testnet.nodes.${window.location.host}${path}`
      );
      break;
    default: // localnet
      if (import.meta.env.VITE_NODE_URL) {
        node = new URL(
          `${import.meta.env.VITE_NODE_URL}${path}`,
          import.meta.url
        );
      } else {
        node = new URL(
          `${import.meta.env.VITE_RUSK_PATH || "/"}${path}`,
          import.meta.url
        );
      }

      break;
  }

  return node;
}

export default makeNodeUrl;
