import { bsc, mainnet, sepolia } from "viem/chains";

/** @type {Tokens} */
export const tokens = {
  mainnet: {
    "BEP-20": {
      chainId: bsc.id,
      migrationContract: "0x32979f040feb67a5223acb0716fe4c7a5956488c#code",
      name: "BEP-20",
      tokenContract: "0xb2bd0749dbe21f623d9baba856d3b0f0e1bfec9c",
    },
    "ERC-20": {
      chainId: mainnet.id,
      migrationContract: "0x36b8e0b938c0172c20e14cc32e7f0e51dcf1084f",
      name: "ERC-20",
      tokenContract: "0x940a2db1b7008b6c776d4faaca729d6d4a4aa551",
    },
  },
  testnet: {
    "BEP-20": {
      chainId: sepolia.id,
      migrationContract: "0x1Bb81fbd735854Ed901aD7Aa1f5F72F64E5841Fc",
      name: "BEP-20",
      tokenContract: "0xC416f5d2AE6BAec2a23f412Df11166afC35CAba2",
    },
    "ERC-20": {
      chainId: sepolia.id,
      migrationContract: "0x81F15Ed1D87A6C840E410D7740D581e36c661640",
      name: "ERC-20",
      tokenContract: "0x92DA9BE2039E818bB78223A6BA7C85CC2b17D8D5",
    },
  },
};
