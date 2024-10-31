const factor = BigInt(1e9);

export default [
  {
    address:
      "c3d4e5f6g7h8i9j0A1B2C3D4E5F6G7H8I9J0a1b2c3d4e5f6g7h8i9j0A1B2C3D4E5F6G7H8",
    balance: {
      shielded: {
        spendable: 400n * factor,
        value: 1234n * factor,
      },
      unshielded: {
        nonce: 1n,
        value: 567n * factor,
      },
    },
  },
  {
    address:
      "B2C3D4E5F6G7H8I9J0a1b2c3d4e5f6g7h8i9j0A1B2C3D4E5F6G7H8I9J0a1b2c3d4e5f6g",
    balance: {
      shielded: {
        spendable: 123n * factor,
        value: 456n * factor,
      },
      unshielded: {
        nonce: 5n,
        value: 123n * factor,
      },
    },
  },
];
