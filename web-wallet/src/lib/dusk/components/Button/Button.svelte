<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";

  import Icon from "../Icon/Icon.svelte";

  /** @type {Boolean} */
  export let active = false;

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {IconProp | Undefined} */
  export let icon = undefined;

  /** @type {ButtonSize} */
  export let size = "normal";

  /** @type {ButtonVariant} */
  export let variant = "primary";

  /** @type {String | Undefined} */
  export let text = undefined;

  /** @type {"button" | "reset" | "submit" | "toggle"} */
  export let type = "button";

  $: classes = makeClassName([
    "dusk-button",
    `dusk-button--type--${type}`,
    `dusk-button--variant--${variant}`,
    `dusk-button--size--${size}`,
    icon && text ? "dusk-icon-button--labeled" : icon ? "dusk-icon-button" : "",
    type === "toggle" && active ? "dusk-button--active" : "",
    className,
  ]);
</script>

<button
  {...$$restProps}
  aria-pressed={type === "toggle" ? active : undefined}
  class={classes}
  on:click
  on:mousedown
  on:mouseup
  type={type === "toggle" ? "button" : type}
>
  {#if icon?.position === "after"}
    {#if text}
      <span class="dusk-button__text">{text}</span>
    {/if}
    <Icon
      className="dusk-button__icon"
      path={icon.path}
      size={icon.size ?? "normal"}
    />
  {:else if icon}
    <Icon
      className="dusk-button__icon"
      path={icon.path}
      size={icon.size ?? "normal"}
    />
    {#if text}
      <span class="dusk-button__text">{text}</span>
    {/if}
  {:else if text}
    <span class="dusk-button__text">{text}</span>
  {/if}
</button>
