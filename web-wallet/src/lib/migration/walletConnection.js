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

// Required project metadata
const projectId = "b5303e1c8374b100fbb7f181884fef28";
const metadata = {
  description: "Dusk Web-Wallet",
  icons: [],
  name: "Dusk Migration",
  url: "https://127.0.0.1:5173/dashboard/",
};

/** @typedef {import("@reown/appkit/networks").AppKitNetwork} AppKitNetwork */
/** @type {[AppKitNetwork, ...AppKitNetwork[]]} */
const networks = [sepolia, bsc, mainnet];

const wagmiAdapter = new WagmiAdapter({
  networks,
  projectId,
});

export const wagmiConfig = wagmiAdapter.wagmiConfig;

reconnect(wagmiConfig);

// Create the Web3 modal with the WAGMI config
export const modal = createAppKit({
  adapters: [wagmiAdapter],
  features: {
    analytics: false,
    onramp: false,
    swaps: false,
  },
  metadata,
  networks,
  projectId,
  themeMode: "dark",
});

// Svelte store to track the current account, and update if a new account is set
// Note that this can change at will by the user outside
// of the app itself
export const account = readable(getAccount(wagmiConfig), (set) => {
  watchAccount(wagmiConfig, {
    onChange(newAccount) {
      set(newAccount);
    },
  });
});

/** @param {*} address */
export const accountBalance = (address) =>
  getBalance(wagmiConfig, {
    address: address,
    blockTag: "latest",
  });

export const walletDisconnect = () => disconnect(wagmiConfig);
