type AccountBalance = {
  nonce: bigint;
  value: bigint;
};

type AddressBalance = {
  spendable: bigint;
  value: bigint;
};

type StakeInfo = Awaited<
  ReturnType<import("$lib/vendor/w3sper.js/src/mod").AccountSyncer["stakes"]>
>[number];

type StakeAmount = StakeInfo["amount"];
