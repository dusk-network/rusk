import { get } from "svelte/store";

import { networkStore } from "$lib/stores";

const { name: networkName } = get(networkStore);

/**
 * Contracts or operations disabled here will stay
 * disabled regardless of other conditions like
 * the sync status.
 *
 * @type {ContractDescriptor[]}
 */
export default [
  {
    enabled: import.meta.env.VITE_FEATURE_TRANSFER === "true",
    id: "send",
    label: "Send",
    operations: [],
  },
  {
    enabled: import.meta.env.VITE_FEATURE_TRANSFER === "true",
    id: "receive",
    label: "Receive",
    operations: [],
  },
  {
    enabled: import.meta.env.VITE_FEATURE_STAKE === "true",
    id: "staking",
    label: "Stake",
    operations: [
      {
        disabled: false,
        id: "stake",
        label: "stake",
        primary: true,
      },
      {
        disabled: false,
        id: "unstake",
        label: "unstake",
        primary: false,
      },
      {
        disabled: false,
        id: "withdraw-rewards",
        label: "withdraw rewards",
        primary: false,
      },
    ],
  },
  {
    enabled: import.meta.env.VITE_FEATURE_ALLOCATE === "true",
    id: "allocate",
    label: "Shield / Unshield",
    operations: [
      {
        disabled: false,
        id: "send",
        label: "send",
        primary: true,
      },
    ],
  },
  {
    // We are missing token configurations for other networks
    // See `src/lib/containers/MigrateContract/MigrateContract.svelte`
    enabled:
      import.meta.env.VITE_FEATURE_MIGRATE === "true" &&
      ["Mainnet", "Testnet"].includes(networkName),
    id: "migrate",
    label: "Migrate",
    operations: [
      {
        disabled: false,
        id: "connect",
        label: "Connect",
        primary: true,
      },
    ],
  },
];
