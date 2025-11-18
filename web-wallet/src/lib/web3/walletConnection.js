import { createAppKit } from "@reown/appkit";
import { WagmiAdapter } from "@reown/appkit-adapter-wagmi";
import { bsc, mainnet, sepolia } from "@reown/appkit/networks";
import {
  disconnect,
  getAccount,
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
const VITE_EVM_BRIDGE_CONTRACT_BLOCK_CREATED = import.meta.env
  .VITE_EVM_BRIDGE_CONTRACT_BLOCK_CREATED;
const VITE_EVM_BRIDGE_BLOCK_EXPLORER_NAME = import.meta.env
  .VITE_EVM_BRIDGE_BLOCK_EXPLORER_NAME;
const VITE_EVM_BRIDGE_BLOCK_EXPLORER_URL = import.meta.env
  .VITE_EVM_BRIDGE_BLOCK_EXPLORER_URL;
const VITE_EVM_BRIDGE_RPC_URL = import.meta.env.VITE_EVM_BRIDGE_RPC_URL;

export const duskEvm = {
  blockExplorers: {
    default: {
      name: VITE_EVM_BRIDGE_BLOCK_EXPLORER_NAME,
      url: VITE_EVM_BRIDGE_BLOCK_EXPLORER_URL,
    },
  },
  contracts: {
    L2StandardBridge: {
      address: VITE_EVM_BRIDGE_CONTRACT_ADDRESS,
      blockCreated: VITE_EVM_BRIDGE_CONTRACT_BLOCK_CREATED,
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
      http: [VITE_EVM_BRIDGE_RPC_URL],
    },
    public: {
      http: [VITE_EVM_BRIDGE_RPC_URL],
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

export async function walletDisconnect() {
  await disconnect(wagmiConfig);
  await modal?.disconnect();
}

window.addEventListener("beforeunload", walletDisconnect);
