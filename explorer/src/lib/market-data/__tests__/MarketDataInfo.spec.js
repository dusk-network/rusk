import { describe, expect, it } from "vitest";

import { MarketDataInfo } from "..";

describe("MarketDataInfo", () => {
  const marketData = {
    currentPrice: {},
    marketCap: {},
  };
  const now = new Date();
  const marketDataInfo = new MarketDataInfo(marketData, now);

  it("should expose the market data and the last update as read-only props", () => {
    expect(() => {
      // @ts-expect-error
      marketDataInfo.data = {};
    }).toThrow();

    expect(() => {
      // @ts-expect-error
      marketDataInfo.lastUpdate = new Date(2010, 3, 4);
    }).toThrow();

    expect(marketDataInfo.data).toStrictEqual(marketData);
    expect(marketDataInfo.lastUpdate).toBe(now);
  });

  it("should expose a method to convert the instance to JSON and a static method to parse it", () => {
    const newMarketDataInfo = MarketDataInfo.parse(marketDataInfo.toJSON());

    expect(newMarketDataInfo).toStrictEqual(marketDataInfo);
  });

  it("should expose a method to convert the data to the format used for storage", () => {
    expect(marketDataInfo.toStorageData()).toStrictEqual({
      data: marketData,
      lastUpdate: now,
    });
  });
});
