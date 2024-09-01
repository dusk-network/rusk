<svelte:options immutable={true} />

<script>
  import { mdiShieldLock, mdiShieldLockOpenOutline } from "@mdi/js";

  import { makeClassName } from "$lib/dusk/string";
  import { Icon } from "$lib/dusk/components";

  /** @type {string | undefined} */
  export let className = undefined;

  /**
   * The percentage of shielded tokens
   * @type {number}
   * */
  export let value;

  $: classes = makeClassName(["usage-indicator", className]);
  $: valueToShow = +value.toFixed(2);
  $: shieldedText = `You have put ${valueToShow}% of your funds in your shielded account`;
  $: unshieldedText = `You have put ${+(100 - valueToShow).toFixed(2)}% of your funds in your unshielded account`;
</script>

<div class={classes}>
  <Icon
    className="usage-indicator__icon"
    data-tooltip-id="main-tooltip"
    data-tooltip-text={shieldedText}
    path={mdiShieldLock}
  />
  <div
    aria-valuemax="100"
    aria-valuemin="0"
    aria-valuenow={valueToShow}
    aria-valuetext={shieldedText}
    class="usage-indicator__meter"
    role="meter"
  >
    <div
      aria-hidden="true"
      class="usage-indicator__meter-bar"
      style:width={`${valueToShow}%`}
    ></div>
  </div>
  <Icon
    className="usage-indicator__icon"
    data-tooltip-id="main-tooltip"
    data-tooltip-text={unshieldedText}
    path={mdiShieldLockOpenOutline}
  />
</div>

<style lang="postcss">
  :global {
    .usage-indicator {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: var(--small-gap);
    }

    .usage-indicator__icon {
      cursor: help;
    }

    .usage-indicator__meter {
      flex: 1;
      height: 1rem;
      background-color: var(--success-color);
      overflow: hidden;
      border-radius: var(--control-border-radius-size);
    }

    .usage-indicator__meter-bar {
      background-color: var(--success-color-variant-dark);
      height: 100%;
    }
  }
</style>
