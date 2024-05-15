<svelte:options immutable={true} />

<script>
  import { DataAlert } from "$lib/components";
  import { Button, Card } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";

  import "./DataCard.css";

  /** @type {Block[] | Transaction[] | Block | Transaction | null}*/
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {String}*/
  export let title;

  /** @type {{action:(e: MouseEvent) => void, disabled:boolean, label: String}}*/
  export let headerButtonDetails;

  /** @type {string | Undefined} */
  export let className = undefined;

  $: classes = makeClassName(["data-card", className]);

  $: hasEmptyData = Array.isArray(data) && data.length === 0;
</script>

<Card className={classes}>
  <header slot="header" class="data-card__header">
    <h1 class="data-card__header-title">{title}</h1>
    <Button
      on:click={headerButtonDetails.action}
      text={headerButtonDetails.label}
      variant="secondary"
      disabled={headerButtonDetails.disabled}
    />
  </header>
  {#if loading && data === null}
    <p>Loading...</p>
  {:else if error || hasEmptyData}
    <DataAlert on:retry {error} />
  {:else if data}
    <slot />
  {/if}
</Card>
