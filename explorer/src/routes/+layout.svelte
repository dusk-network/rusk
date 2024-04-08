<script>
  import { Navbar } from "$lib/components";
  import "../style/main.css";

  /**
   * @param {Boolean} bool
   */
  const toggleScroll = (bool) => {
    if (!document.body) {
      return;
    }

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
<main class="explorer">
  <slot />
</main>
