<script>
  import { Tooltip } from "$lib/dusk/components";
  import { Footer, Navbar } from "$lib/components";
  import { appStore } from "$lib/stores";
  import "../style/main.css";

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

<Tooltip id="main-tooltip" />
