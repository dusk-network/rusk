<script>
  import { Tooltip } from "$lib/dusk/components";
  import { Footer, Navbar } from "$lib/components";
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
