<script>
  import { Button, Textbox } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import "./TextboxAndButton.css";

  /** @type {string} */
  export let placeholder = "";

  /** @type {number}*/
  export let fieldWidth = 150;

  /** @type {number}*/
  export let expandedFieldWidth = 300;

  /** @type {string} */
  export let value = "";

  /** @type {IconProp | undefined}*/
  export let icon = undefined;

  /** @type {string | undefined} */
  export let buttonText = undefined;

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {TextboxTypes}*/
  export let type = "text";

  /** @type {Textbox} */
  let inputElement;

  export function focus() {
    inputElement?.focus();
  }
  export function select() {
    inputElement?.select();
  }

  const BREAKPOINT = 1024;
  const FULL_WIDTH = "100%";

  const fieldWidthProps = {
    max: expandedFieldWidth,
    min: fieldWidth,
  };

  /** @type {HTMLElement} */
  let textField;

  /** @type {Number} */
  let clientWidth;

  /** @type {Boolean}*/
  let extended = false;

  /** @type {String} */
  let width;

  function openField() {
    extended = true;
  }

  function closeField() {
    extended = false;
  }

  /**
   * @param {boolean} expanded
   */
  function setSearchFieldWidth(expanded) {
    if (clientWidth < BREAKPOINT) {
      width = FULL_WIDTH;
    } else {
      expandedFieldWidth = fieldWidthProps.max;
      fieldWidth = fieldWidthProps.min;
      width = expanded ? `${expandedFieldWidth}px` : `${fieldWidth}px`;
    }
  }

  /**
   * @param { ResizeObserverEntry[] } entries
   */
  function handleResize(entries) {
    clientWidth = entries[0].target.clientWidth;

    setSearchFieldWidth(extended);
  }

  $: setSearchFieldWidth(extended);

  $: classes = makeClassName(["textbox-button", className]);

  onMount(() => {
    const resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(document.documentElement);

    return () => resizeObserver.disconnect();
  });
</script>

<div
  class={classes}
  style="width: {width};"
  bind:this={textField}
  on:focusin={openField}
  on:focusout={closeField}
>
  <Textbox
    className="textbox-button__input"
    bind:this={inputElement}
    bind:value
    {type}
    required
    {placeholder}
    on:blur
    on:focus
    on:paste
  />
  <Button
    on:click
    type="submit"
    {icon}
    text={buttonText}
    size="normal"
    variant="secondary"
  />
</div>
