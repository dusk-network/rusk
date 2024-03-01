type MnemonicEncryptInfo = {
  data: Uint8Array;
  iv: Uint8Array;
  salt: Uint8Array;
};

type WalletStakeInfo = {
  amount: number;
  reward: number;
  has_key: boolean;
  has_staked: boolean;
};
