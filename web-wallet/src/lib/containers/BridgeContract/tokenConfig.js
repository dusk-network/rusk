import { mainnet, sepolia } from "viem/chains";

export const tokens = {
  devnet: {
    "ERC-20": {
      bridgeContract: "0x81F15Ed1D87A6C840E410D7740D581e36c661640",
      chainId: sepolia.id,
      name: "ERC-20",
      tokenContract: "0x92DA9BE2039E818bB78223A6BA7C85CC2b17D8D5",
    },
  },
  mainnet: {
    "ERC-20": {
      bridgeContract: "0x36b8e0b938c0172c20e14cc32e7f0e51dcf1084f",
      chainId: mainnet.id,
      name: "ERC-20",
      tokenContract: "0x940a2db1b7008b6c776d4faaca729d6d4a4aa551",
    },
  },
  testnet: {
    "ERC-20": {
      bridgeContract: "0x81F15Ed1D87A6C840E410D7740D581e36c661640",
      chainId: sepolia.id,
      name: "ERC-20",
      tokenContract: "0x92DA9BE2039E818bB78223A6BA7C85CC2b17D8D5",
    },
  },
};
