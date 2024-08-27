import { bsc, mainnet, sepolia } from "viem/chains";

const mainnetMigrateContract = import.meta.env.VITE_MIGRATE_CONTRACT;

/** @type {Tokens} */
export const tokens = {
  mainnet: {
    "BEP-20": {
      chainId: bsc.id,
      contract: "0xb2bd0749dbe21f623d9baba856d3b0f0e1bfec9c",
      migrationContract: mainnetMigrateContract,
      name: "BEP-20",
    },
    "ERC-20": {
      chainId: mainnet.id,
      contract: "0x940a2db1b7008b6c776d4faaca729d6d4a4aa551",
      migrationContract: mainnetMigrateContract,
      name: "ERC-20",
    },
  },
  testnet: {
    "BEP-20": {
      chainId: sepolia.id,
      contract: "0xC416f5d2AE6BAec2a23f412Df11166afC35CAba2",
      migrationContract: "0x9f5d1c067710fc6ed49a6444afd69b64799a57b6",
      name: "BEP-20",
    },
    "ERC-20": {
      chainId: sepolia.id,
      contract: "0x92DA9BE2039E818bB78223A6BA7C85CC2b17D8D5",
      migrationContract: "0x63fd2B12034e108BCe73e8832b7dabC8bd67f738",
      name: "ERC-20",
    },
  },
};
