class InvalidPasswordError extends Error {
  constructor() {
    super("Wrong password");
    this.name = "InvalidPasswordError";
  }
}

export default InvalidPasswordError;
