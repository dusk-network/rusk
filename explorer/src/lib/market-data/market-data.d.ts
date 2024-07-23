type MarketData = {
  currentPrice: Record<string, number>;
  marketCap: Record<string, number>;
};

type MarketDataStorage = {
  data: MarketData;
  lastUpdate: Date;
};
