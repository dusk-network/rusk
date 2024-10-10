type GpsLocation = {
  lat: string;
  lon: string;
};

type NodeLocationsCount = {
  node: GpsLocation;
  count: number;
};

type ReverseGeocodeData = {
  city: string;
  country: string;
  count: number;
};
