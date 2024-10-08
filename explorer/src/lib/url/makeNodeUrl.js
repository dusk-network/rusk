/**
 * Constructs a node URL based on the subdomain
 *
 * @returns {URL} nodeUrl
 */
function makeNodeUrl() {
  const domains = window.location.hostname.split(".");

  let node;

  switch (domains[0]) {
    case "apps": // mainnet
      node = new URL(
        `${window.location.protocol}nodes.${window.location.host}`
      );
      break;
    case "devnet":
      node = new URL(
        `${window.location.protocol}devnet.nodes.${window.location.host}`
      );
      break;
    case "testnet":
      node = new URL(
        `${window.location.protocol}testnet.nodes.${window.location.host}`
      );
      break;
    default: // localnet
      node = new URL(
        `${import.meta.env.VITE_RUSK_PATH || "/"}`,
        import.meta.url
      );

      if (import.meta.env.VITE_NODE_URL) {
        node = new URL(import.meta.env.VITE_NODE_URL, import.meta.url);
      }

      break;
  }

  return node;
}

export default makeNodeUrl;
