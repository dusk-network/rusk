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
    id: "stake",
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
        id: "claim-rewards",
        label: "claim rewards",
        primary: false,
      },
    ],
  },
  {
    enabled: import.meta.env.VITE_FEATURE_ALLOCATE === "true",
    id: "allocate",
    label: "Allocate",
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
    enabled: import.meta.env.VITE_FEATURE_MIGRATE === "true",
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
