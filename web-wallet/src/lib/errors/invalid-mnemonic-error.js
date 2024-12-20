class InvalidMnemonicError extends Error {
  constructor() {
    super("Invalid mnemonic");
    this.name = "InvalidMnemonicError";
  }
}

export default InvalidMnemonicError;
