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
    id: "transfer",
    label: "Transact",
    operations: [
      {
        disabled: false,
        id: "send",
        label: "Send",
        primary: true,
      },
      {
        disabled: false,
        id: "receive",
        label: "Receive",
        primary: false,
      },
    ],
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
];
