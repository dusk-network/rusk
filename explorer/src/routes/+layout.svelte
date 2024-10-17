<script>
  import { onMount } from "svelte";

  import { Tooltip } from "$lib/dusk/components";
  import { Footer, Navbar } from "$lib/components";
  import { appStore } from "$lib/stores";
  import { duskAPI } from "$lib/services";
  import "../style/main.css";

  onMount(async () => {
    appStore.setNodeInfo(await duskAPI.getNodeInfo());
  });

  /**
   * @param {Boolean} bool
   */
  const toggleScroll = (bool) => {
    if (bool) {
      document.body.style.overflow = "auto";
    } else {
      document.body.style.overflow = "hidden";
    }
  };

  appStore.subscribe(({ darkMode }) => {
    document.documentElement.classList.toggle("dark", darkMode);
  });

  const { hasTouchSupport } = $appStore;
</script>

<header>
  <Navbar on:toggleMenu={(e) => toggleScroll(e.detail)} />
</header>
<main id="explorer">
  <slot />
</main>
<footer>
  <Footer />
</footer>

<Tooltip defaultDelayShow={hasTouchSupport ? 0 : undefined} id="main-tooltip" />
