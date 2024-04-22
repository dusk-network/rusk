<svelte:options immutable={true} />

<script>
  import { AppAnchor, AppImage } from "$lib/components";
  import { Anchor } from "$lib/dusk/components";
  import "./Footer.css";
</script>

<div class="footer">
  <div class="footer__copyright">
    <span>© 2018 - 2024 Dusk Network B.V. All Rights Reserved.</span>
  </div>
  <div class="footer__links">
    <Anchor
      href="https://dusk.network/privacy-policy"
      className="footer__links-link">Privacy policy</Anchor
    >
    <Anchor
      href="https://dusk.network/terms-of-use"
      className="footer__links-link">Terms of use</Anchor
    >
  </div>
  <AppAnchor href="https://dusk.network" className="footer__logo">
    <picture>
      <source media="(max-width:768px)" srcset="/dusk_logo_icon.svg" />
      <source media="(min-width:769px)" srcset="/dusk_logo.svg" />
      <AppImage
        src="/dusk_logo.svg"
        alt="Dusk Logo"
        sizes="(max-width:768px) 20px, 86px"
        className="footer__logo-image"
      />
    </picture>
  </AppAnchor>
</div>
