<script>
  import { AppImage } from "$lib/components";
  import { settingsStore } from "$lib/stores";

  const { darkMode } = $settingsStore;
</script>

<header>
  <AppImage
    src={darkMode ? "/dusk_logo_light.svg" : "/dusk_logo.svg"}
    alt="Dusk Logo"
    width="129"
    height="31"
  />
</header>

<slot />
