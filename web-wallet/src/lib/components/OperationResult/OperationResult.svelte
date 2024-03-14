<svelte:options immutable={true} />

<script>
  import { mdiCheckDecagramOutline } from "@mdi/js";

  import { makeClassName } from "$lib/dusk/string";
  import { Icon, Suspense } from "$lib/dusk/components";

  import { AppAnchorButton } from "..";

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

<Suspense
  className={classes}
  {errorMessage}
  gap="large"
  {pendingMessage}
  waitFor={operation}
>
  <svelte:fragment slot="success-content" let:result>
    <Icon path={mdiCheckDecagramOutline} size="large" />
    <span>{successMessage}</span>
    <slot name="success-content" {result} />
    <AppAnchorButton
      href="/dashboard"
      on:click={handleGoHomeClick}
      variant="tertiary"
      text="HOME"
    />
  </svelte:fragment>
  <AppAnchorButton
    href="/dashboard"
    on:click={handleGoHomeClick}
    slot="error-extra-content"
    text="HOME"
    variant="tertiary"
  />
</Suspense>

<style lang="postcss">
  :global {
    .operation-result {
      padding: 1.5em 0;

      .dusk-anchor-button {
        width: 100%;
      }
    }
  }
</style>
