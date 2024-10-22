<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import {
    calculateAdaptiveCharCount,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import {
    mdiContentCopy,
    mdiPlusBoxOutline,
    mdiSwapHorizontal,
    mdiTimerSand,
  } from "@mdi/js";
  import { Button, Icon, ProgressBar } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { handlePageClick } from "$lib/dusk/ui-helpers/handlePageClick";

  import Overlay from "./Overlay.svelte";

  import "./AddressPicker.css";

  /** @type {import("$lib/vendor/w3sper.js/src/mod").Profile | null} */
  export let currentProfile;

  /** @type {import("$lib/vendor/w3sper.js/src/mod").Profile[]} */
  export let profiles;

  /** @type {boolean} */
  export let isAddingProfile = false;

  /** @type {string|undefined} */
  export let className = undefined;

  $: classes = makeClassName(["address-picker", className]);

  const dispatch = createEventDispatcher();

  let expanded = false;

  /** @type {HTMLMenuElement} */
  let addressOptionsMenu;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  function closeDropDown() {
    expanded = false;
  }

  /** @type {import("svelte/elements").KeyboardEventHandler<HTMLDivElement>} */
  function handleDropDownKeyDown(event) {
    if (event.key === "Enter" || event.key === " ") {
      copyCurrentAddress();
    }

    if (event.key === "Escape") {
      closeDropDown();
    }
  }

  function copyCurrentAddress() {
    navigator.clipboard.writeText(currentAddress);
    toast("success", "Address copied", mdiContentCopy);
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  // Scrolls the address options menu to top on addresses change
  $: if (profiles && addressOptionsMenu) {
    addressOptionsMenu.scrollTo(0, 0);
  }
  $: currentAddress = currentProfile ? currentProfile.address.toString() : "";
</script>

{#if expanded}
  <Overlay />
{/if}

<div
  use:handlePageClick={{ callback: closeDropDown, enabled: expanded }}
  class={classes}
  class:address-picker--expanded={expanded}
>
  <div
    class="address-picker__trigger"
    role="button"
    tabindex="0"
    aria-haspopup="true"
    aria-expanded={expanded}
    on:keydown={handleDropDownKeyDown}
  >
    <Button disabled variant="secondary" icon={{ path: mdiSwapHorizontal }} />

    <p class="address-picker__current-address">
      {middleEllipsis(currentAddress, calculateAdaptiveCharCount(screenWidth))}
    </p>
    <Button
      aria-label="Copy Address"
      className="address-picker__copy-address-button"
      icon={{ path: mdiContentCopy }}
      on:click={copyCurrentAddress}
      variant="secondary"
    />
  </div>

  {#if expanded}
    <div class="address-picker__drop-down">
      <hr />
      <menu
        class="address-picker__address-options"
        bind:this={addressOptionsMenu}
      >
        {#each profiles as profile (profile)}
          <li
            class="address-picker__address"
            class:address-picker__address--selected={profile === currentProfile}
          >
            <button
              class="address-picker__address-option-button"
              tabindex="0"
              type="button"
              role="menuitem"
              on:click={() => {
                dispatch("setCurrentProfile");
                closeDropDown();
              }}>{profile.address.toString()}</button
            >
          </li>
        {/each}
      </menu>
      <hr />
      {#if isAddingProfile}
        <div class="address-picker__generating-address-wrapper">
          <Icon path={mdiTimerSand} />
          <p>Generating <b>Address</b></p>
        </div>
        <ProgressBar />
      {:else}
        <Button
          tabindex="0"
          className="address-picker__generate-address-button"
          icon={{ path: mdiPlusBoxOutline }}
          text="Generate Address"
          on:click={(event) => {
            event.preventDefault();
            dispatch("generateProfile");
          }}
        />
      {/if}
    </div>
  {/if}
</div>
