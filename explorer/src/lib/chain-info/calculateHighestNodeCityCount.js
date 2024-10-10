/**
 * Aggregates location data and returns the city with the highest total node count.
 *
 * @param {ReverseGeocodeData[]} data
 * @returns {ReverseGeocodeData}
 */
const calculateHighestNodeCityCount = (data) => {
  const cityMap = new Map();

  data.forEach(({ city, country, count }) => {
    if (cityMap.has(city)) {
      const existingEntry = cityMap.get(city);
      existingEntry.count += count;
    } else {
      cityMap.set(city, { city, count, country });
    }
  });

  /** @type {ReverseGeocodeData} */
  let highestCity = { city: "", count: 0, country: "" };

  cityMap.forEach((value) => {
    if (!highestCity || value.count > highestCity.count) {
      highestCity = value;
    }
  });

  return highestCity;
};

export default calculateHighestNodeCityCount;
