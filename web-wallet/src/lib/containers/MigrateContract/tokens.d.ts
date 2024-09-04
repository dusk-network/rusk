type HexString = `0x${string}`;

type TokenInfo = {
  chainId: number;
  contract: HexString;
  migrationContract: HexString;
  name: TokenNames;
};

type NetworkTokens = {
  "BEP-20": TokenInfo;
  "ERC-20": TokenInfo;
};

type Tokens = {
  mainnet: NetworkTokens;
  testnet: NetworkTokens;
};

type TokenNames = "BEP-20" | "ERC-20";
