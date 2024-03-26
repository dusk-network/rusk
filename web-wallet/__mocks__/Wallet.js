class Wallet {
  constructor(seed) {
    this.seed = seed;
    this.wasm = {};
  }

  static get networkBlockHeight() {
    return Promise.resolve(0);
  }

  seed;
  wasm;

  async history() {}
  async getBalance() {}
  async getPsks() {}
  async stake() {}
  async stakeAllow() {}
  async stakeInfo() {}
  async reset() {}
  async sync() {}
  async transfer() {}
  async unstake() {}
  async withdrawReward() {}
}

export default Wallet;
