import { duskAPI } from "$lib/services";
import { countBy } from "lamb";

const locationKey = "locations-data";
const geoKey = "geo-data";

/**
 * @param {"reading" | "storing"} action
 * @param {unknown} err
 */
const logStoreError = (action, err) =>
  /* eslint-disable-next-line no-console */
  console.error(`Error while ${action} local data: %s`, err);

/**
 *
 * @param {string} key
 * @returns {*|null}
 */
function getStorage(key) {
  try {
    const storedData = localStorage.getItem(key);

    return storedData ? JSON.parse(storedData) : null;
  } catch (err) {
    logStoreError("reading", err);
    return null;
  }
}

/**
 * @param {string} key
 * @param {*} info
 */
function setStorage(key, info) {
  try {
    localStorage.setItem(key, JSON.stringify(info));
  } catch (err) {
    logStoreError("storing", err);
  }
}

/** @type {(source: Array<{lat:number, lon:number}>) => Record<string,number>} */
const countByLatLon = countBy((obj) => `${obj.lat},${obj.lon}`);

/**
 * @param {Record<string,number>} data
 * @returns {NodeLocationsCount[]}
 */
const countOccurrences = (data) =>
  Object.entries(data).map(([key, value]) => {
    return {
      count: value,
      node: {
        lat: key.split(",")[0],
        lon: key.split(",")[1],
      },
    };
  });

/**
 * Filters the given data to find the integer latitude and longitude pairs
 * that have the highest total count, returning all associated nodes.
 *
 * @param {NodeLocationsCount[]} data
 * @returns {NodeLocationsCount[]}
 */
const filterHighestCountSum = (data) => {
  const map = new Map();

  data.forEach((item) => {
    const lat = Math.trunc(Number(item.node.lat));
    const lon = Math.trunc(Number(item.node.lon));
    const key = `${lat},${lon}`;

    // Initialize or update the map with the node details
    if (!map.has(key)) {
      map.set(key, { nodes: [item], sum: item.count });
    } else {
      const existingEntry = map.get(key);
      existingEntry.nodes.push(item); // Collect all relevant nodes
      existingEntry.sum += item.count; // Sum the counts
    }
  });

  // Find the entry with the maximum sum
  let maxEntry = null;

  for (const entry of map.values()) {
    if (!maxEntry || entry.sum > maxEntry.sum) {
      maxEntry = entry;
    }
  }

  return maxEntry.nodes;
};

/**
 * @param {NodeLocationsCount[]} data
 * @returns
 */
const retrieveAndSetLocationData = async (data) => {
  try {
    const res = await duskAPI.getReverseGeocodeData(data);
    setStorage(locationKey, res);
  } catch (err) {
    /* eslint-disable-next-line no-console */
    console.error("Error while retriving reverse geocode data", err);
  }
};

/**
 * @param {NodeLocationsCount[]} nodeData
 * @param {ReverseGeocodeData} locationStore
 * @returns {Promise<ReverseGeocodeData>}
 */
const checkLocationStore = async (nodeData, locationStore) => {
  if (!locationStore) {
    await retrieveAndSetLocationData(nodeData);
  }

  return Promise.resolve(getStorage(locationKey));
};

/**
 * @returns {Promise<ReverseGeocodeData>}
 */
const geoData = () => {
  return duskAPI.getNodeLocations().then(async (data) => {
    const geoStoredData = getStorage(geoKey);
    const locationStoredData = getStorage(locationKey);
    const highestNodeLocation = filterHighestCountSum(
      countOccurrences(countByLatLon(data))
    );

    if (geoStoredData) {
      const isNewNodeLocationsData =
        JSON.stringify(data) !== JSON.stringify(geoStoredData);

      if (isNewNodeLocationsData) {
        setStorage(geoKey, data);
        await retrieveAndSetLocationData(highestNodeLocation);
      }

      return await checkLocationStore(highestNodeLocation, locationStoredData);
    } else {
      setStorage(geoKey, data);

      return await checkLocationStore(highestNodeLocation, locationStoredData);
    }
  });
};

export default geoData;
