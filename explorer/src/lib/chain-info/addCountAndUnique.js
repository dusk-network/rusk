/**
 * Returns an array of unique objects, sorted by the frequency of their occurrence,
 * with an additional count property representing the number of times
 * that city-country combination appeared in the input data.
 *
 * @param {Omit<NodeLocation, "count">[]} data
 * @returns {NodeLocation[]}
 */
function addCountAndUnique(data) {
  const countMap = data.reduce((locationMap, location) => {
    const key = `${location.city},${location.country}`;

    if (!locationMap[key]) {
      locationMap[key] = { ...location, count: 0 };
    }
    locationMap[key].count += 1;
    return locationMap;
  }, /** @type {{ [key: string]: NodeLocation }} */ ({}));

  // Return only unique city-country combinations
  return Object.values(countMap).sort((a, b) => b.count - a.count);
}

export default addCountAndUnique;
