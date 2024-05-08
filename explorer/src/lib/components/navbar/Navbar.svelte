<script>
  import { AppAnchor, AppImage, SearchNotification } from "$lib/components";
  import { SearchField } from "$lib/containers";
  import { NavList, Select } from "$lib/dusk/components";
  import { createEventDispatcher, tick } from "svelte";
  import "./Navbar.css";

  /** @type {Number}*/
  let clientWidth;

  /** @type {number} */
  let offset;

  /** @type {boolean} */
  let hidden = true;

  /** @type {boolean} */
  let showSearchNotification = false;

  /** @type {*} */
  let notificationData;

  const navigation = [
    {
      link: "/",
      title: "Chain Info",
    },
    {
      link: "/blocks",
      title: "Blocks",
    },
    {
      link: "/transactions",
      title: "Transactions",
    },
  ];

  const networks = [
    {
      label: "testnet",
      value: `${import.meta.env.VITE_DUSK_TESTNET_NODE}`,
    },
    {
      label: "devnet",
      value: `${import.meta.env.VITE_DUSK_DEVNET_NODE}`,
    },
  ];

  const dispatch = createEventDispatcher();

  async function createEmptySpace() {
    await tick();
    offset = document.getElementsByClassName(
      "dusk-navbar__menu--search-notification"
    )[0]?.clientHeight;
  }
</script>

<svelte:window bind:innerWidth={clientWidth} />

<nav
  style={showSearchNotification
    ? `margin-bottom: ${offset}px`
    : "margin-bottom: 0"}
  class="dusk-navbar"
  class:dusk-navbar--menu-hidden={hidden}
>
  <AppAnchor href="/" className="dusk-navbar__logo">
    <AppImage
      src="/dusk_logo.svg"
      alt="Dusk Logo"
      sizes="(max-width: 1024px) 86px, 129px"
    />
  </AppAnchor>

  <button
    on:click={() => {
      hidden = !hidden;
      dispatch("toggleMenu", hidden);
    }}
    type="button"
    class="dusk-navbar__toggle"
    aria-controls="dusk-navbar-menu"
    aria-expanded={!hidden}
  >
    {#if hidden}
      <svg
        class="dusk-navbar__toggle-svg"
        aria-hidden="true"
        viewBox="0 0 41 14"
        xmlns="http://www.w3.org/2000/svg"
      >
        <line y1="1" x2="40.8" y2="1" stroke-width="2"></line>
        <line y1="12.9854" x2="40.8" y2="12.9854" stroke-width="2"></line>
      </svg>
    {:else}
      <svg
        class="dusk-navbar__toggle-svg"
        aria-hidden="true"
        viewBox="0 0 32 31"
        xmlns="http://www.w3.org/2000/svg"
      >
        <line
          x1="1.29289"
          y1="30.2929"
          x2="30.1428"
          y2="1.44294"
          stroke-width="2"
        ></line>
        <line
          x1="1.70711"
          y1="1.29289"
          x2="30.5571"
          y2="30.1428"
          stroke-width="2"
        ></line>
      </svg>
    {/if}
  </button>

  <div
    class="dusk-navbar__menu"
    class:dusk-navbar__menu--hidden={hidden}
    id="dusk-navbar-menu"
  >
    <Select className="dusk-navbar__menu--network" options={networks} />
    <NavList className="dusk-navbar__menu--links" {navigation} />
    <div class="dusk-navbar__menu--search">
      <SearchField
        on:invalid={(e) => {
          notificationData = e.detail;
          showSearchNotification = true;
          createEmptySpace();
        }}
      />

      {#if showSearchNotification}
        <SearchNotification
          data={notificationData}
          on:close={() => {
            showSearchNotification = false;
          }}
          className="dusk-navbar__menu--search-notification"
        />
      {/if}
    </div>
  </div>
</nav>
