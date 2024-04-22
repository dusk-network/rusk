<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { Select } from "$lib/dusk/components";
  import Badges from "./Badges.svelte";
  import Balances from "./Balances.svelte";
  import Buttons from "./Buttons.svelte";
  import Cards from "./Cards.svelte";
  import Checkboxes from "./Checkboxes.svelte";
  import ProgressBars from "./ProgressBars.svelte";
  import Selects from "./Selects.svelte";
  import Steppers from "./Steppers.svelte";
  import Switches from "./Switches.svelte";
  import Tabboxes from "./Tabboxes.svelte";
  import Textboxes from "./Textboxes.svelte";
  import Throbbers from "./Throbbers.svelte";
  import Toasts from "./Toasts.svelte";
  import Tooltips from "./Tooltips.svelte";
  import Wizards from "./Wizards.svelte";

  /** @type {Record<string, import("svelte").ComponentType>} */
  const componentsMap = {
    Badges: Badges,
    Balances: Balances,
    Buttons: Buttons,
    Cards: Cards,
    Checkboxes: Checkboxes,
    "Progress bars": ProgressBars,
    Selects: Selects,
    Steppers: Steppers,
    Switches: Switches,
    Tabboxes: Tabboxes,
    Textboxes: Textboxes,
    Throbbers: Throbbers,
    Toasts: Toasts,
    Tooltips: Tooltips,
    Wizards: Wizards,
  };

  const componentsOptions = Object.keys(componentsMap);

  let selectedSection = componentsOptions[0];
</script>

<section>
  <header>
    <h2>COMPONENTS SHOWCASE</h2>
    <Select options={componentsOptions} bind:value={selectedSection} />
  </header>

  {#key selectedSection}
    <div in:fade>
      <svelte:component this={componentsMap[selectedSection]} />
    </div>
  {/key}
</section>

<style lang="postcss">
  section {
    overflow: hidden;
    display: flex;
    flex-direction: column;

    & > header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 1em;
      padding: 0.5em;
    }

    & > div {
      flex: 1;
      margin-top: 2em;
      overflow-y: auto;

      & > :global(section) {
        display: flex;
        align-items: center;
        justify-content: space-around;
        flex-wrap: wrap;
        gap: 1em;
        margin: 1.5em 0;
      }
    }
  }
</style>
