<svelte:options immutable={true} />

<script>
  import "css-doodle";
  import Animation from "./Animation.svelte";
  import { AppAnchorButton } from "$lib/components";
  import { mdiKeyOutline, mdiRestore, mdiWalletOutline } from "@mdi/js";
</script>

<section class="setup-content">
  <div class="setup-content__wrapper">
    <h2 class="h1 setup-heading">
      Unlocking the Future: <br />
      Your <mark>DUSK</mark> Native Wallet
    </h2>

    <div class="setup-content__options">
      <div class="setup-content__options-group">
        <AppAnchorButton
          href="/setup/create"
          text="Create new wallet"
          icon={{ path: mdiWalletOutline }}
        />
        <AppAnchorButton
          href="/setup/restore/"
          variant="tertiary"
          text="Restore Wallet"
          icon={{ path: mdiRestore }}
        />
      </div>
      <Animation />
      <AppAnchorButton
        href="/login"
        text="Access wallet"
        icon={{ path: mdiKeyOutline }}
      />
    </div>
  </div>
</section>

<footer class="setup-footer">
  <span
    >Web Wallet v{import.meta.env.APP_VERSION} ({import.meta.env
      .APP_BUILD_INFO})</span
  >
</footer>

<style lang="postcss">
  .setup-content {
    flex: 1;

    &__wrapper {
      display: flex;
      flex-direction: column;
      gap: var(--large-gap);
      height: 100%;
    }

    &__options {
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      height: 100%;

      &-group {
        display: flex;
        flex-direction: column;
        gap: var(--default-gap);
      }
    }
  }

  .setup-heading {
    position: relative;
  }

  .setup-footer {
    font-size: 0.75em;
  }
</style>
