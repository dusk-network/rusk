<svelte:options immutable={true} />

<script>
  import { AppAnchor, AppImage, AppSource } from "$lib/components";
  import { Anchor } from "$lib/dusk/components";
  import { appStore } from "$lib/stores";
  import "./Footer.css";

  $: ({ darkMode } = $appStore);
</script>

<div class="footer">
  <div class="footer__copyright">
    <span>© 2018 - 2024 Dusk Network B.V. All Rights Reserved.</span>
    <span
      >Explorer v{import.meta.env.APP_VERSION} ({import.meta.env
        .APP_BUILD_INFO})</span
    >
  </div>

  <div class="footer__links-test">
    <div class="footer__links">
      <Anchor
        onSurface={false}
        href="https://dusk.network/privacy-policy"
        className="footer__links-link">Privacy policy</Anchor
      >
      <Anchor
        onSurface={false}
        href="https://dusk.network/terms-of-use"
        className="footer__links-link">Terms of use</Anchor
      >
    </div>
    <AppAnchor href="https://dusk.network" className="footer__logo">
      <picture>
        <AppSource media="(max-width:768px)" srcset="/dusk_logo_icon.svg" />
        <AppImage
          src={darkMode ? "/dusk_logo_light.svg" : "/dusk_logo.svg"}
          alt="Dusk Logo"
          sizes="(max-width:768px) 20px, 86px"
          className="footer__logo-image"
        />
      </picture>
    </AppAnchor>
  </div>
</div>
