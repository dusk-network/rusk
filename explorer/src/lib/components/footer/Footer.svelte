<svelte:options immutable={true} />

<script>
  import { AppAnchor, AppImage } from "$lib/components";
  import { Anchor } from "$lib/dusk/components";
  import { appStore } from "$lib/stores";
  import "./Footer.css";

  const hostedExplorers = [
    {
      label: "Mainnet",
      url: "https://apps.dusk.network/explorer",
    },
    {
      label: "Testnet",
      url: "https://apps.testnet.dusk.network/explorer",
    },
    {
      label: "Devnet",
      url: "https://apps.devnet.dusk.network/explorer",
    },
  ];

  $: ({ darkMode } = $appStore);
</script>

<div class="footer">
  <div class="footer__copyright-container">
    <p>© 2018 - 2024 Dusk Network B.V. All Rights Reserved.</p>
    <p>
      Explorer v{import.meta.env.APP_VERSION} ({import.meta.env.APP_BUILD_INFO})
    </p>
  </div>
  <div class="footer__explorers-links-container">
    {#each hostedExplorers as explorer (explorer.url)}
      {#if !location.href.includes(explorer.url)}
        <Anchor onSurface={false} href={explorer.url} className="footer__link"
          >{explorer.label}</Anchor
        >
      {/if}
    {/each}
  </div>

  <div class="footer__misc-links-container">
    <Anchor
      onSurface={false}
      href="https://dusk.network/privacy-policy"
      className="footer__link">Privacy Policy</Anchor
    >
    <Anchor
      onSurface={false}
      href="https://dusk.network/terms-of-use"
      className="footer__link">Terms of Use</Anchor
    >
  </div>

  <AppAnchor href="https://dusk.network" className="footer__logo">
    <picture>
      <AppImage
        src={darkMode ? "/dusk_logo_light.svg" : "/dusk_logo.svg"}
        alt="Dusk Logo"
        sizes="(max-width:768px) 20px, 86px"
      />
    </picture>
  </AppAnchor>
</div>
