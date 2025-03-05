import { createAppKit } from "@reown/appkit";
import { WagmiAdapter } from "@reown/appkit-adapter-wagmi";
// eslint-disable-next-line import/no-unresolved
import { bsc, mainnet, sepolia } from "@reown/appkit/networks";
import {
  disconnect,
  getAccount,
  getBalance,
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
const projectId = import.meta.env.VITE_REOWN_PROJECT_ID || "";

/** @typedef {import("@reown/appkit/networks").AppKitNetwork} AppKitNetwork */
/** @type {[AppKitNetwork, ...AppKitNetwork[]]} */
const networks = [sepolia, bsc, mainnet];

const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId,
});

export const wagmiConfig = wagmiAdapter.wagmiConfig;

reconnect(wagmiConfig);

// Create the Reown App Kit modal
export const modal = createAppKit({
  adapters: [wagmiAdapter],
  features: {
    analytics: false,
    onramp: false,
    swaps: false,
  },
  networks,
  projectId,
  themeMode: "dark",
});

export const account = readable(getAccount(wagmiConfig), (set) =>
  watchAccount(wagmiConfig, { onChange: set })
);

/** @param {`0x${string}`} address */
export const accountBalance = (address) =>
  getBalance(wagmiConfig, {
    address: address,
    blockTag: "latest",
  });

export async function walletDisconnect() {
  disconnect(wagmiConfig);
  await modal?.disconnect();
}

window.addEventListener("beforeunload", walletDisconnect);
