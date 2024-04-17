<svelte:options immutable={true} />

<script>
  import { makeClassName } from "$lib/dusk/string";
    import { onMount } from "svelte";
  import "./Textbox.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {TextboxTypes} */
  export let type = "text";

  /** @type {String | Number} */
  export let value = type === "number" ? 0 : "";

  /**
   * @type {HTMLInputElement | HTMLTextAreaElement}
   */
  let inputElement;

  /**@type {Boolean}*/
  let isInputDisabled;

  export function focus() {
    inputElement?.focus();
  }

  export function select() {
    inputElement?.select();
  }

  $: classes = makeClassName([
    "dusk-textbox",
    `dusk-textbox-${type}`,
    className,
  ]);

  /**
   * Needed, as the value cannot be bound to the input element
   * when the type is set dynamically
   * @param {Event & {currentTarget: EventTarget & HTMLInputElement}} event
   */
  function handleInput(event) {
    const target = event.currentTarget;

    value = target.type === "number" ? target.valueAsNumber : target.value;
  }

  onMount(()=>{
    isInputDisabled = inputElement.disabled;
  })
</script>

{#if type === "multiline"}
  <textarea
    {...$$restProps}
    class={classes}
    bind:this={inputElement}
    bind:value
    on:input
  />
{:else}
<label 
  class={classes}
  class:dusk-textbox--disabled={isInputDisabled}
>
  {#if $$slots.before}
    <slot name="before"/>
  {/if}
  <input
    class="dusk-textbox__input"
    {...$$restProps}
    {type}
    {value}
    bind:this={inputElement}
    on:blur
    on:input={handleInput}
    on:input
    on:keydown
    on:paste
  />
  {#if $$slots.after}
    <slot name="after"/>
  {/if}
</label>
{/if}
