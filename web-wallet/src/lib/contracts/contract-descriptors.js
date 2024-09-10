/**
 * Contracts or operations disabled here will stay
 * disabled regardless of other conditions like
 * the sync status.
 *
 * @type {ContractDescriptor[]}
 */
export default [
  {
    disabled:
      import.meta.env.VITE_CONTRACT_TRANSFER_DISABLED &&
      import.meta.env.VITE_CONTRACT_TRANSFER_DISABLED === "true",
    id: "send",
    label: "Send",
    operations: [],
  },
  {
    disabled:
      import.meta.env.VITE_CONTRACT_TRANSFER_DISABLED &&
      import.meta.env.VITE_CONTRACT_TRANSFER_DISABLED === "true",
    id: "receive",
    label: "Receive",
    operations: [],
  },
  {
    disabled:
      import.meta.env.VITE_CONTRACT_STAKE_DISABLED &&
      import.meta.env.VITE_CONTRACT_STAKE_DISABLED === "true",
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
    disabled:
      import.meta.env.VITE_CONTRACT_ALLOCATE_DISABLED &&
      import.meta.env.VITE_CONTRACT_ALLOCATE_DISABLED === "true",
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
    disabled:
      import.meta.env.VITE_CONTRACT_MIGRATE_DISABLED &&
      import.meta.env.VITE_CONTRACT_MIGRATE_DISABLED === "true",
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
