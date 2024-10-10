import { afterAll, afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { apiGeocodeLocations, apiNodeLocations  } from "$lib/mock-data";
import { calculateHighestNodeCityCount } from "$lib/chain-info";
import { geoData, duskAPI } from "..";

const mockGeoStoredData = apiNodeLocations.data;
const mockLocationStoredData = {
    city: "North Bergen",
    count: 16,
    country: "United States of America",
};
const mockHighestNodeLocation = [
    {
        count: 6,
        node: {
            lat: "1.32123",
            lon: "103.695"
        }
    },
    {
        count: 2,
        node: {
            lat: "1.35208",
            lon: "103.82"
        }
    },
    {
        count: 1,
        node: {
            lat: "1.28967",
            lon: "103.85"
        }
    },
    {
        count: 1,
        node: {
            lat: "1.27989",
            lon: "103.849"
        }
    },
    {
        count: 1,
        node: {
            lat: "1.28223",
            lon: "103.851"
        }
    }
]

describe("geoData", () => {
    const getItemSpy = vi.spyOn(Storage.prototype, 'getItem')
    const setItemSpy = vi.spyOn(Storage.prototype, 'setItem')

    beforeEach(() => {       
        vi.spyOn(duskAPI, "getNodeLocations").mockResolvedValue(mockGeoStoredData);
        vi.spyOn(duskAPI, "getReverseGeocodeData").mockResolvedValue(mockLocationStoredData);
    });

    afterEach(()=>{
        localStorage.clear();
        getItemSpy.mockClear();
        setItemSpy.mockClear();
    })
    
    it.only("should retrieve and set geo data from local storage when it exists", async () => {
        const result = await geoData();

        expect(duskAPI.getNodeLocations).toHaveBeenCalledOnce();
        expect(getItemSpy).toHaveBeenCalledWith("geo-data");
        console.log(getItemSpy.mock.results)
        //expect(getItemSpy.mock.results).toHaveReturnedWith(null);
    
        expect(getItemSpy).toHaveBeenCalledWith("locations-data");
        expect(getItemSpy).toHaveReturnedWith(null);

        expect(setItemSpy).toHaveBeenCalledWith("geo-data", JSON.stringify(mockGeoStoredData));
        expect(setItemSpy).toHaveBeenCalledWith("locations-data", JSON.stringify(mockLocationStoredData));
        
       
        expect(result).toEqual(mockLocationStoredData);
    });
    
    it("should fetch and store reverse geocode data if not available", async () => {   
        const result = await geoData();
    
        expect(duskAPI.getReverseGeocodeData).toHaveBeenCalledWith(mockHighestNodeLocation);
    
        // Ensure localStorage.setItem is called to save the reverse geocode data
        expect(setItemSpy).toHaveBeenCalledWith("locations-data", JSON.stringify(mockLocationStoredData));
    
        // Ensure the result is the fetched reverse geocode data
        expect(result).toEqual(mockLocationStoredData);
    });
});
