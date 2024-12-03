<svelte:options immutable={true} />

<script>
  import { DataAlert } from "$lib/components";
  import { Button, Card, ProgressBar } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";

  import "./DataCard.css";

  /** @type {Block[] | Transaction[] | HostProvisioner[] | Block | Transaction | null}*/
  export let data;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {String}*/
  export let title;

  /** @type {{action:(e: MouseEvent) => void, disabled: boolean, label: String, variant?: ButtonVariant } | undefined}*/
  export let headerButtonDetails = undefined;

  /** @type {string | Undefined} */
  export let className = undefined;

  $: classes = makeClassName(["data-card", className]);
  $: hasEmptyData = Array.isArray(data) && data.length === 0;
</script>

<Card className={classes}>
  <header slot="header" class="data-card__header">
    <h1 class="data-card__header-title">{title}</h1>
    {#if headerButtonDetails}
      <Button
        on:click={headerButtonDetails.action}
        text={headerButtonDetails.label}
        variant={headerButtonDetails.variant || "tertiary"}
        disabled={headerButtonDetails.disabled}
      />
    {/if}
  </header>
  {#if loading && !data}
    <ProgressBar ariaLabel="Loading" className="data-card__progress-bar" />
  {:else if error || hasEmptyData}
    <DataAlert on:retry {error} />
  {:else if data}
    <div class="data-card__content">
      <slot />
    </div>
  {/if}
</Card>
