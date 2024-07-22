import { compose, updateKey } from "lamb";

/** @returns {MarketData} */
const jsonToMarketData = compose(
  updateKey("lastUpdate", (v) => new Date(v)),
  JSON.parse
);

class MarketDataInfo {
  /** @type {MarketData} */
  #data;

  /** @type {Date} */
  #lastUpdate;

  /** @param {string} json */
  static parse(json) {
    const { data, lastUpdate } = jsonToMarketData(json);

    return new MarketDataInfo(data, lastUpdate);
  }

  /**
   *
   * @param {MarketData} data
   * @param {Date} lastUpdate
   */
  constructor(data, lastUpdate) {
    this.#data = data;
    this.#lastUpdate = lastUpdate;
  }

  get data() {
    return this.#data;
  }

  get lastUpdate() {
    return this.#lastUpdate;
  }

  toJSON() {
    return JSON.stringify(this.toStorageData());
  }

  toStorageData() {
    return {
      data: this.#data,
      lastUpdate: this.#lastUpdate,
    };
  }
}

export default MarketDataInfo;
