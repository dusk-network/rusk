<script>
  // @ts-nocheck

  import { geoNaturalEarth1, geoPath } from "d3-geo";
  import { uniquesBy } from "lamb";
  import dataset from "./world-map.json";

  import "./WorldMap.css";

  /** @type {Array<{lat: number, lon:number}> | Error}*/
  export let nodes;

  /** @type {import("d3-geo").GeoProjection}*/
  const projection = geoNaturalEarth1();

  /** @type {import("d3-geo").GeoPath<any, import("d3").GeoPermissibleObjects>} */
  const path = geoPath(projection);

  /** Function will return an empty array if an error is passed to it */
  /** Function will always receive an empty array, an array of objects or an error */
  const getUniqueMarkers = uniquesBy(({ lat, lon }) => `${lat}x${lon}`);
  const nodeLocations = getUniqueMarkers(nodes);
</script>

<svg id="nodes-world-map" viewBox="0 0 954 477">
  <defs>
    <pattern
      id="vertical-lines"
      patternUnits="userSpaceOnUse"
      width="5"
      height="10"
    >
      <line x1="2.5" y1="0" x2="2.5" y2="10" stroke="black" stroke-width="1" />
    </pattern>
  </defs>
  {#each dataset.features as data, index (index)}
    <path d={path(data)} style="fill:url(#vertical-lines)" />
  {/each}
  {#if nodeLocations.length}
    {#each nodeLocations as marker, index (index)}
      <circle
        cx={projection([marker.lon, marker.lat])[0]}
        cy={projection([marker.lon, marker.lat])[1]}
        r="3"
      ></circle>
    {/each}
  {/if}
</svg>
