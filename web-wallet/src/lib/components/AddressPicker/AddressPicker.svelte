<svelte:options immutable={true} />

<script>
  import { createEventDispatcher } from "svelte";
  import {
    calculateAdaptiveCharCount,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import { mdiAlertOutline, mdiChevronDown, mdiContentCopy } from "@mdi/js";
  import { Icon } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { handlePageClick } from "$lib/dusk/ui-helpers/handlePageClick";

  import Overlay from "./Overlay.svelte";

  import "./AddressPicker.css";

  /** @type {Profile | null} */
  export let currentProfile;

  /** @type {Profile[]} */
  export let profiles;

  /** @type {string|undefined} */
  export let className = undefined;

  const dispatch = createEventDispatcher();

  let expanded = false;
  let innerWidth = 0;

  /** @type {HTMLMenuElement} */
  let addressOptionsMenu;

  function closeDropDown() {
    expanded = false;
  }

  function toggleDropDown() {
    expanded = !expanded;
  }

  /** @type {import("svelte/elements").MouseEventHandler<HTMLDivElement>} */
  function handleTriggerClick() {
    toggleDropDown();
  }

  /** @type {import("svelte/elements").KeyboardEventHandler<HTMLDivElement>} */
  function handleTriggerKeyDown(event) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      toggleDropDown();
    }
    if (event.key === "Escape") {
      closeDropDown();
    }
    if (event.key === "ArrowDown" && !expanded) {
      event.preventDefault();
      toggleDropDown();
    }
  }

  /**
   * @param {Profile} profile
   */
  function handleProfileSelect(profile) {
    dispatch("setCurrentProfile", { profile });
    closeDropDown();
  }

  /**
   * @param {string} address
   */
  async function handleCopyAddress(address) {
    try {
      await navigator.clipboard.writeText(address);
      toast("success", "Address copied", mdiContentCopy);
    } catch (err) {
      toast(
        "error",
        err instanceof Error && err.name === "NotAllowedError"
          ? "Clipboard access denied"
          : "Failed to copy address",
        mdiAlertOutline
      );
    }
  }

  /**
   * @param {KeyboardEvent & { currentTarget: EventTarget & HTMLButtonElement }} event
   * @param {Profile} profile
   */
  function handleProfileKeyDown(event, profile) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      handleProfileSelect(profile);
    }
    if (event.key === "Escape") {
      event.preventDefault();
      closeDropDown();
    }
  }

  $: classes = makeClassName(["address-picker", className]);

  // Scrolls the address options menu to top on addresses change
  $: if (profiles && addressOptionsMenu) {
    addressOptionsMenu.scrollTo(0, 0);
  }

  $: displayedAddress = currentProfile
    ? `Profile ${profiles.indexOf(currentProfile) + 1}${profiles.indexOf(currentProfile) === 0 ? " (Default)" : ""}`
    : "No profile selected";

  // Calculate adaptive character count based on screen width
  $: charCount = calculateAdaptiveCharCount(innerWidth);
</script>

{#if expanded}
  <Overlay />
{/if}

<svelte:window bind:innerWidth />

<div
  use:handlePageClick={{
    callback: closeDropDown,
    enabled: expanded,
  }}
  class={classes}
  class:address-picker--expanded={expanded}
>
  <div
    class="address-picker__trigger"
    role="button"
    tabindex="0"
    aria-haspopup="menu"
    aria-expanded={expanded}
    aria-label="Select address profile"
    on:click={handleTriggerClick}
    on:keydown={handleTriggerKeyDown}
  >
    <span class="address-picker__current-profile">{displayedAddress}</span>
    <Icon path={mdiChevronDown} className="address-picker__chevron" />
  </div>

  {#if expanded}
    <div class="address-picker__drop-down">
      <ul
        class="address-picker__address-options"
        bind:this={addressOptionsMenu}
        role="menu"
        aria-label="Profile selection"
      >
        {#each profiles as profile (profile)}
          <li
            class="address-picker__profile"
            class:address-picker__profile--selected={profile === currentProfile}
            role="none"
          >
            <div class="address-picker__profile-container">
              <button
                class="address-picker__profile-button"
                tabindex="0"
                type="button"
                role="menuitem"
                aria-current={profile === currentProfile ? "true" : "false"}
                on:click={() => handleProfileSelect(profile)}
                on:keydown={(event) => handleProfileKeyDown(event, profile)}
              >
                <div class="address-picker__profile-header">
                  Profile {profiles.indexOf(profile) + 1}
                  {#if profile === currentProfile}
                    <span class="address-picker__current-indicator"
                      >(Current)</span
                    >
                  {/if}
                </div>
                <div class="address-picker__addresses">
                  <div class="address-picker__address-row">
                    <span class="address-picker__address-label"
                      >Public Account</span
                    >
                    <span class="address-picker__address-value"
                      >{middleEllipsis(
                        profile.account.toString(),
                        charCount
                      )}</span
                    >
                  </div>
                  <div class="address-picker__address-row">
                    <span class="address-picker__address-label"
                      >Shielded Account</span
                    >
                    <span class="address-picker__address-value"
                      >{middleEllipsis(
                        profile.address.toString(),
                        charCount
                      )}</span
                    >
                  </div>
                </div>
              </button>
              <div class="address-picker__copy-buttons">
                <button
                  class="address-picker__copy-button"
                  type="button"
                  title="Copy public address"
                  on:click={() => handleCopyAddress(profile.account.toString())}
                >
                  <Icon path={mdiContentCopy} size="small" />
                </button>
                <button
                  class="address-picker__copy-button"
                  type="button"
                  title="Copy shielded address"
                  on:click={() => handleCopyAddress(profile.address.toString())}
                >
                  <Icon path={mdiContentCopy} size="small" />
                </button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
