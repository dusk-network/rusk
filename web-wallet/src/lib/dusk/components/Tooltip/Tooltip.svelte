<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { writable } from "svelte/store";
  import {
    computePosition,
    flip,
    inline,
    offset as setOffset,
    shift,
  } from "@floating-ui/dom";

  import { makeClassName } from "$lib/dusk/string";

  import "./Tooltip.css";

  /** @type {String | Undefined} */
  export let className = undefined;

  /**
   * Default delay in ms before hiding the tooltip.
   * @type {Number}
   */
  export let defaultDelayHide = 0;

  /**
   * Default delay in ms before showing the tooltip.
   * @type {Number}
   */
  export let defaultDelayShow = 500;

  /**
   * Default offset from the target element.
   * @type {Number}
   */
  export let defaultOffset = 10;

  /**
   * Preferred default placement.
   * @type {import("@floating-ui/dom").Side}
   */
  export let defaultPlace = "top";

  /**
   * Tooltip's default type.
   * @type {TooltipType}
   */
  export let defaultType = "info";

  /**
   * ID of the tooltip element.
   * @type {String}
   */
  export let id;

  /** @type {Number} */
  let timeoutID = 0;

  /** @type {HTMLDivElement} */
  let tooltipElement;

  const state = writable({
    delayHide: defaultDelayHide,
    delayShow: defaultDelayShow,
    offset: defaultOffset,
    place: defaultPlace,
    text: "",
    type: defaultType,
    visible: false,
    x: 0,
    y: 0,
  });

  const intersectionObserver = new IntersectionObserver((entries, observer) => {
    if (entries[0].intersectionRatio <= 0 || !entries[0].target.isConnected) {
      clearTimeout(timeoutID);
      state.set({
        ...$state,
        text: "",
        visible: false,
      });
      observer.disconnect();
    }
  });

  /** @type {import("svelte/elements").KeyboardEventHandler<HTMLElement>} */
  function handleKeydown(event) {
    if (event.key === "Escape") {
      handleTooltipHide(event);
    }
  }

  // @ts-ignore
  function handleTooltipHide(event) {
    const { tooltipDelayHide, tooltipId } = event.target.dataset;

    if (tooltipId !== id) {
      return;
    }

    clearTimeout(timeoutID);

    const delayHide = tooltipDelayHide ? +tooltipDelayHide : defaultDelayHide;
    const newState = {
      ...$state,
      text: "",
      visible: false,
    };

    intersectionObserver.unobserve(event.target);

    if (delayHide) {
      timeoutID = window.setTimeout(() => {
        event.target.removeAttribute("aria-described-by");
        state.set(newState);
      }, delayHide);
    } else {
      event.target.removeAttribute("aria-described-by");
      state.set(newState);
    }
  }

  // @ts-ignore
  async function handleTooltipShow(event) {
    const {
      tooltipDelayShow,
      tooltipDisabled,
      tooltipId,
      tooltipOffset = defaultOffset,
      tooltipPlace = defaultPlace,
      tooltipText = "",
      tooltipType = defaultType,
    } = event.target.dataset;

    if (tooltipId !== id || tooltipDisabled === "true") {
      return;
    }

    clearTimeout(timeoutID);
    state.set({ ...$state, text: tooltipText });
    intersectionObserver.observe(event.target);

    const { placement, x, y } = await computePosition(
      event.target,
      tooltipElement,
      {
        middleware: [
          setOffset({ mainAxis: +tooltipOffset }),
          inline(),
          flip({ fallbackAxisSideDirection: "start" }),
          shift(),
        ],
        placement: tooltipPlace,
        strategy: "fixed",
      }
    );

    // We consider only "top", "right", "bottom" and "left" for now.
    // The extra parenthesis are needed to force the cast for the type checker.
    // eslint-disable-next-line no-extra-parens
    const place = /** @type {import("@floating-ui/dom").Side} */ (
      placement.replace(/-.+$/, "")
    );

    const newState = {
      ...$state,
      place,
      type: tooltipType,
      visible: true,
      x,
      y,
    };

    const delayShow = tooltipDelayShow ? +tooltipDelayShow : defaultDelayShow;

    if (delayShow) {
      timeoutID = window.setTimeout(() => {
        if (event.target && event.target.isConnected) {
          setAriaDescription(event.target);
          state.set(newState);
        }
      }, delayShow);
    } else {
      setAriaDescription(event.target);
      state.set(newState);
    }
  }

  /** @param {HTMLElement} target */
  function setAriaDescription(target) {
    document
      .querySelector(`[aria-described-by="${id}"]`)
      ?.removeAttribute("aria-described-by");
    target.setAttribute("aria-described-by", id);
  }

  onDestroy(() => {
    intersectionObserver.disconnect();
  });

  $: ({
    place = defaultPlace,
    text,
    type = defaultType,
    visible,
    x,
    y,
  } = $state);

  $: classes = makeClassName([
    "dusk-tooltip",
    `dusk-tooltip-${place}`,
    `dusk-tooltip-${type}`,
    className,
  ]);
</script>

<svelte:body
  on:focusin|capture={handleTooltipShow}
  on:focusout|capture={handleTooltipHide}
  on:keydown|capture={handleKeydown}
  on:mouseenter|capture={handleTooltipShow}
  on:mouseleave|capture={handleTooltipHide}
/>
<div
  {...$$restProps}
  bind:this={tooltipElement}
  aria-hidden={!visible}
  class={classes}
  {id}
  role="tooltip"
  style:left={`${x}px`}
  style:top={`${y}px`}
>
  {text}
</div>
