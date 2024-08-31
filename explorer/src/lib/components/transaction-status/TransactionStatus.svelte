<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";

  import { Badge } from "$lib/dusk/components";

  import "./TransactionStatus.css";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {string} */
  export let errorMessage;

  /** @type {boolean} */
  export let showErrorTooltip = false;

  $: classes = makeClassName(["transaction-status", className]);

  /** @type {import("svelte").ComponentProps<Badge>} */
  $: props = errorMessage
    ? {
        ...(showErrorTooltip
          ? {
              "data-tooltip-id": "main-tooltip",
              "data-tooltip-text": errorMessage,
              "data-tooltip-type": "error",
            }
          : null),
        text: "failed",
        variant: "error",
      }
    : {
        text: "success",
        variant: "success",
      };
</script>

<Badge className={classes} {...props} />
