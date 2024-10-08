<script>
  // @ts-nocheck
  import { geoNaturalEarth1, geoPath } from "d3-geo";
  import { scaleSqrt } from "d3-scale";
  import dataset from "./world-map.json";

  import "./WorldMap.css";

  /** @type {NodeLocation[] | null}*/
  export let nodes;

  /** @type {string} */
  export let stroke = "black";

  /** @type {import("d3-geo").GeoProjection}*/
  const projection = geoNaturalEarth1().scale(195).center([0, 8]);

  /** @type {import("d3-geo").GeoPath<any, import("d3").GeoPermissibleObjects>} */
  const path = geoPath(projection);

  const radius = scaleSqrt().domain([0, 100]).range([0, 20]);
</script>

<svg class="nodes-world-map" viewBox="0 0 954 477">
  <defs>
    <pattern
      id="vertical-lines"
      patternUnits="userSpaceOnUse"
      width="5"
      height="10"
    >
      <line x1="2.5" y1="0" x2="2.5" y2="10" {stroke} stroke-width="1" />
    </pattern>
  </defs>
  {#each dataset.features as data, index (index)}
    <path d={path(data)} style="fill:url(#vertical-lines)" />
  {/each}
  <g>
    {#each nodes ? nodes : [] as marker (`${marker.lon}x${marker.lat}`)}
      <circle
        class="nodes-world-map__location"
        cx={projection([marker.lon, marker.lat])[0]}
        cy={projection([marker.lon, marker.lat])[1]}
        data-tooltip-id="main-tooltip"
        data-tooltip-text={`${marker.country} - ${marker.city} - ${marker.count} ${marker.count === 1 ? "node" : "nodes"}`}
        data-tooltip-place="top"
        data-tooltip-type="info"
        r={radius(marker.count)}
      ></circle>
    {/each}
  </g>
</svg>
