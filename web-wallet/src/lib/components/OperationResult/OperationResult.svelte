<svelte:options immutable={true} />

<script>
  import { ErrorDetails, Icon, Throbber } from "$lib/dusk/components";
  import { mdiCheckDecagramOutline, mdiCloseThick } from "@mdi/js";
  import { makeClassName } from "$lib/dusk/string";
  import { AppAnchorButton } from "$lib/components";

  /** @type {string|undefined} */
  export let className = undefined;

  /** @type {Promise<any>} */
  export let operation;

  /** @type {string} */
  export let errorMessage = "Operation failed";

  /** @type {Function|undefined} */
  export let onBeforeLeave = undefined;

  /** @type {string} */
  export let pendingMessage = "";

  /** @type {string} */
  export let successMessage = "Operation completed";

  /** @param {Event} event */
  function handleGoHomeClick(event) {
    event.preventDefault();

    onBeforeLeave && onBeforeLeave();
  }

  $: classes = makeClassName(["operation-result", className]);
</script>

<div class={classes}>
  {#await operation}
    <Throbber />
    <span>{pendingMessage}</span>
  {:then result}
    <Icon path={mdiCheckDecagramOutline} size="large" />
    <span>{successMessage}</span>
    <slot name="success-content" {result} />
    <AppAnchorButton
      href="/dashboard"
      on:click={handleGoHomeClick}
      variant="tertiary"
      text="HOME"
    />
  {:catch error}
    <Icon path={mdiCloseThick} size="large" />
    <ErrorDetails {error} summary={errorMessage} />
    <slot name="error-content" />
    <AppAnchorButton
      href="/dashboard"
      on:click={handleGoHomeClick}
      variant="tertiary"
      text="HOME"
    />
  {/await}
</div>

<style lang="postcss">
  .operation-result {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--large-gap);
    padding: 1.5em 0;

    :global(.dusk-anchor-button) {
      width: 100%;
    }
  }
</style>
