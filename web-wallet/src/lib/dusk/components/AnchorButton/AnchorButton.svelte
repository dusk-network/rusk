<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";

  import Anchor from "../Anchor/Anchor.svelte";
  import Icon from "../Icon/Icon.svelte";

  import "./AnchorButton.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {Boolean} */
  export let disabled = false;

  /** @type {String} */
  export let href;

  /** @type {IconProp | Undefined} */
  export let icon = undefined;

  /** @type {ButtonSize} */
  export let size = "normal";

  /** @type {ButtonVariant} */
  export let variant = "secondary";

  /** @type {String | Undefined} */
  export let text = undefined;

  $: classes = makeClassName([
    "dusk-anchor-button",
    `dusk-anchor-button--variant--${variant}`,
    `dusk-anchor-button--size--${size}`,
    disabled ? "dusk-anchor-button--disabled" : "",
    icon && text ? "dusk-icon-button-labeled" : icon ? "dusk-icon-button" : "",
    className,
  ]);
</script>

<Anchor
  {...$$restProps}
  aria-disabled={disabled}
  className={classes}
  {href}
  on:click
  tabindex={disabled ? "-1" : $$restProps.tabindex ?? undefined}
>
  {#if icon?.position === "after"}
    {#if text}
      <span class="dusk-anchor-button__text">{text}</span>
    {/if}
    <Icon
      className="dusk-anchor-button__icon"
      path={icon.path}
      size={icon.size}
    />
  {:else if icon}
    <Icon
      className="dusk-anchor-button__icon"
      path={icon.path}
      size={icon.size}
    />
    {#if text}
      <span class="dusk-anchor-button__text">{text}</span>
    {/if}
  {:else if text}
    <span class="dusk-anchor-button__text">{text}</span>
  {/if}
</Anchor>
