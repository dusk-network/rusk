class MismatchedWalletError extends Error {
  constructor() {
    super("Mismatched wallet address or no existing wallet");
    this.name = "MismatchedWalletError";
  }
}

export default MismatchedWalletError;
