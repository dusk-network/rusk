<svelte:options immutable={true} />

<script>
  import { Alert } from "$lib/components";
  import { Button, Card } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";

  import "./DataCard.css";

  /** @type {Block[] | Transaction[] | Block | Transaction}*/
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {String}*/
  export let title;

  /** @type {{action:(e: MouseEvent) => void, label: String}}*/
  export let button;

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Boolean} */
  let hasEmptyData;

  $: classes = makeClassName(["data-card", className]);

  $: {
    hasEmptyData = Array.isArray(data) && data.length === 0;
  }
</script>

<Card className={classes}>
  <header slot="header" class="data-card__header">
    <h1 class="data-card__header-title">{title}</h1>
    <Button
      on:click={button.action}
      text={button.label}
      variant="secondary"
    />
  </header>
  {#if loading && data === null}
    <p>Loading...</p>
  {:else if error || hasEmptyData}
    <Alert on:retry {error} {hasEmptyData} />
  {:else if data}
    <slot />
  {/if}
</Card>
