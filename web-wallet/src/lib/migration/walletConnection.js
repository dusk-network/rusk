import { createWeb3Modal, defaultWagmiConfig } from "@web3modal/wagmi";
import {
  disconnect,
  getAccount,
  getBalance,
  reconnect,
  watchAccount,
} from "@wagmi/core";
import { readable } from "svelte/store";
import { bsc, mainnet, sepolia } from "viem/chains";

// Required project metadata
const projectId = "b5303e1c8374b100fbb7f181884fef28";
const metadata = {
  description: "Dusk Web-Wallet",
  icons: [],
  name: "Dusk Migration",
  url: "https://127.0.0.1:5173/dashboard/",
};

/** @type {[import("viem").Chain, import("viem").Chain, import("viem").Chain]} */
const chains = [sepolia, bsc, mainnet];

export const wagmiConfig = defaultWagmiConfig({
  auth: { email: false },
  chains,
  metadata,
  projectId,
});
reconnect(wagmiConfig);

// Create the Web3 modal with the WAGMI config
export const modal = createWeb3Modal({
  allowUnsupportedChain: false,
  enableAnalytics: false,
  enableOnramp: false,
  enableSwaps: false,
  projectId,
  themeMode: "dark",
  wagmiConfig,
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
  getBalance(wagmiConfig, { address: address, blockTag: "latest" });

export const walletDisconnect = () => disconnect(wagmiConfig);
