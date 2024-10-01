<svelte:options immutable={true} />

<script>
  import { mdiWrenchClock } from "@mdi/js";

  import { AnchorButton } from "$lib/dusk/components";
  import { IconHeadingCard } from "$lib/containers/Cards";
</script>

<section class="maintenance">
  <h2 class="h1">
    Unleash <mark>RWA</mark> and<br />
    Decentralized <mark>Finance</mark>
  </h2>
  <div class="maintenance">
    <IconHeadingCard
      tag="article"
      gap="medium"
      heading="Wallet maintenance"
      icons={[mdiWrenchClock]}
    >
      <p>
        Dusk web-wallet is undergoing some essential maintenance and will return
        soon. Need an alternative? Rusk wallet is our terminal wallet
        application.
      </p>
    </IconHeadingCard>
  </div>
  <footer class="maintenance-footer">
    <AnchorButton
      href="https://github.com/dusk-network/rusk/blob/master/rusk-wallet/src/bin/README.md"
      variant="tertiary"
      text="Rusk Wallet"
    />
  </footer>
</section>

<style lang="postcss">
  .maintenance,
  .maintenance-footer {
    display: flex;
    flex-direction: column;
  }

  .maintenance {
    height: 100%;
    overflow-y: auto;
    gap: var(--large-gap);
  }

  .maintenance-footer {
    gap: var(--default-gap);
  }
</style>
