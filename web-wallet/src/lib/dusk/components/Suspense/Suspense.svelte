<svelte:options immutable={true} />

<script>
  import { getErrorFrom } from "$lib/dusk/error";
  import { makeClassName } from "$lib/dusk/string";

  import { ErrorAlert, ErrorDetails, Throbber } from "..";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let errorMessage = "Error";

  /** @type {SuspenceErrorVariant} */
  export let errorVariant = "alert";

  /** @type {GapSize} */
  export let gap = "normal";

  /** @type {string} */
  export let pendingMessage = "";

  /** @type {Promise<any>} */
  export let waitFor;

  $: classes = makeClassName([
    "dusk-suspense",
    gap !== "normal" ? `dusk-suspense--${gap}-gap` : "",
    className,
  ]);
</script>

<div {...$$restProps} class={classes}>
  {#await waitFor}
    <slot name="pending-content">
      <Throbber className="dusk-suspense__throbber" />
      <span class="dusk-suspense__pending-message">{pendingMessage}</span>
    </slot>
  {:then result}
    <slot name="success-content" {result} />
  {:catch thrownError}
    {@const error = getErrorFrom(thrownError)}
    {@const ErrorComponent =
      errorVariant === "alert" ? ErrorAlert : ErrorDetails}
    {@const extraProps = errorVariant === "alert" ? { gap } : {}}
    <slot name="error-content" {error}>
      <svelte:component
        this={ErrorComponent}
        className="dusk-suspense__error"
        {error}
        summary={errorMessage}
        {...extraProps}
      />
      <slot name="error-extra-content" {error} />
    </slot>
  {/await}
</div>
