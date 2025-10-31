import { createAppKit } from "@reown/appkit";
import { WagmiAdapter } from "@reown/appkit-adapter-wagmi";
import { bsc, mainnet, sepolia } from "@reown/appkit/networks";
import {
  disconnect,
  getAccount,
  getBalance,
  http,
  reconnect,
  watchAccount,
} from "@wagmi/core";
import { readable } from "svelte/store";

/**
 * @constant {string} projectId - The ID of the project, sourced from an environment variable.
 *
 * @description
 * This constant retrieves the project ID from the environment variable `VITE_REOWN_PROJECT_ID`.
 * If the environment variable is not set, it defaults to an empty string. This behavior is not ideal
 * because an empty `projectId` will cause the modal initialization to fail.
 *
 * While this issue typically arises due to a developer error (e.g., forgetting to set the environment variable),
 * resolving it properly requires a broader refactor of the codebase to better handle missing or invalid `projectId` values.
 *
 * Additionally, when the `projectId` is missing, the "migrate" functionality will not be accessible in the UI,
 * effectively hiding the "broken" flow. Therefore, while the error can occur, users are unlikely to encounter it.
 *
 * **To improve:**
 * Consider implementing a mechanism to ensure `VITE_REOWN_PROJECT_ID` is always defined during the build or
 * runtime processes, potentially throwing a clear error during startup if it is missing.
 */
const VITE_REOWN_PROJECT_ID = import.meta.env.VITE_REOWN_PROJECT_ID || "";
const VITE_EVM_BRIDGE_CONTRACT_ADDRESS = import.meta.env
  .VITE_EVM_BRIDGE_CONTRACT_ADDRESS;

const duskEvm = {
  blockExplorers: {
    default: {
      name: "Dusk EVM Explorer",
      url: "https://explorer.testnet.evm.dusk.network",
    },
  },
  contracts: {
    L2StandardBridge: {
      address: VITE_EVM_BRIDGE_CONTRACT_ADDRESS,
      blockCreated: 5882,
    },
  },
  id: 310,
  name: "DuskEVM",
  nativeCurrency: {
    decimals: 18,
    name: "Dusk",
    symbol: "DUSK",
  },
  rpcUrls: {
    default: {
      http: ["https://rpc.testnet.evm.dusk.network"],
    },
    public: {
      http: ["https://rpc.testnet.evm.dusk.network"],
    },
  },
};

/** @typedef {import("@reown/appkit/networks").AppKitNetwork} AppKitNetwork */
/** @type {[AppKitNetwork, ...AppKitNetwork[]]} */
const networks = [sepolia, bsc, mainnet, duskEvm];

const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId: VITE_REOWN_PROJECT_ID,
  transports: {
    [duskEvm.id]: http(duskEvm.rpcUrls.default.http[0]),
  },
});

export const wagmiConfig = wagmiAdapter.wagmiConfig;

console.log("Wagmi Config:", wagmiConfig);

reconnect(wagmiConfig);

// Create the Reown App Kit modal
export const modal = createAppKit({
  adapters: [wagmiAdapter],
  defaultNetwork: duskEvm,
  features: {
    analytics: false,
    onramp: false,
    swaps: false,
  },
  networks,
  projectId: VITE_REOWN_PROJECT_ID,
  themeMode: "dark",
});

export const account = readable(getAccount(wagmiConfig), (set) =>
  watchAccount(wagmiConfig, { onChange: set })
);

/**
 * @param {`0x${string}`} address
 * @returns {Promise<import("@wagmi/core").GetBalanceReturnType>} balance as bigint or null if no address provided
 */
export async function getAccountBalance(address) {
  return await getBalance(wagmiConfig, {
    address,
    chainId: duskEvm.id,
  });
}

export async function walletDisconnect() {
  await disconnect(wagmiConfig);
  await modal?.disconnect();
}

window.addEventListener("beforeunload", walletDisconnect);
