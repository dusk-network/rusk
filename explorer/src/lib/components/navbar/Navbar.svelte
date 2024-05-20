<script>
  import { createEventDispatcher, tick } from "svelte";
  import { mdiClose, mdiMenu } from "@mdi/js";
  import { afterNavigate } from "$app/navigation";

  import { Button, NavList, Select } from "$lib/dusk/components";
  import { AppAnchor, AppImage, SearchNotification } from "$lib/components";
  import { SearchField } from "$lib/containers";
  import { appStore } from "$lib/stores";

  import "./Navbar.css";

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

  const dispatch = createEventDispatcher();

  async function createEmptySpace() {
    await tick();
    offset = document.getElementsByClassName(
      "dusk-navbar__menu--search-notification"
    )[0]?.clientHeight;
  }

  /**
   * @param {Event} e
   */
  function handleNetworkChange(e) {
    // @ts-ignore
    appStore.setNetwork(e.target.value);
  }

  afterNavigate(() => {
    hidden = true;
    dispatch("toggleMenu", hidden);
    showSearchNotification = false;
  });

  $: ({ networks } = $appStore);
</script>

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
  <Button
    aria-controls="dusk-navbar-menu"
    aria-expanded={!hidden}
    className="dusk-navbar__toggle"
    icon={{ path: hidden ? mdiMenu : mdiClose, size: "large" }}
    on:click={() => {
      hidden = !hidden;
      dispatch("toggleMenu", hidden);
    }}
  />
  <div
    class="dusk-navbar__menu"
    class:dusk-navbar__menu--hidden={hidden}
    id="dusk-navbar-menu"
  >
    <Select
      className="dusk-navbar__menu--network"
      on:change={handleNetworkChange}
      options={networks}
    />
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
