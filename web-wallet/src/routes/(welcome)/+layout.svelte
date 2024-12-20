<script>
  import { ErrorDetails, Throbber } from "$lib/dusk/components";
  import { AppAnchor, AppImage, Banner } from "$lib/components";
  import { networkStore, settingsStore } from "$lib/stores";

  const { darkMode } = $settingsStore;

  let retryKey = 0;

  /** @param {Event} event */
  function handleRetry(event) {
    event.preventDefault();

    retryKey ^= 1;
  }
</script>

<header>
  <AppImage
    src={darkMode ? "/dusk_logo_light.svg" : "/dusk_logo.svg"}
    alt="Dusk Logo"
    width="129"
    height="31"
  />
</header>
{#key retryKey}
  {#await networkStore.connect().then(() => networkStore.init())}
    <div class="welcome-layout__loading">
      <Throbber />
      <strong>CONNECTING TO THE NETWORK</strong>
    </div>
  {:then}
    <slot />
  {:catch error}
    <Banner
      className="welcome-layout__error"
      title="Error while trying to connect to the network"
      variant="error"
    >
      <p>
        The Web Wallet is unable to connect to the network.<br />
        This may be a temporary issue.<br />
        Please try again in a few minutes by clicking <AppAnchor
          href="/setup"
          on:click={handleRetry}>retry</AppAnchor
        >.
      </p>
      <ErrorDetails {error} summary="Error details" />
    </Banner>
  {/await}
{/key}

<style lang="postcss">
  :global {
    .welcome-layout__error,
    .welcome-layout__loading {
      margin-top: 10dvh;
    }

    .welcome-layout__error > .banner__content {
      gap: var(--medium-gap);
    }

    .welcome-layout__loading {
      width: 100%;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--medium-gap);
    }
  }
</style>
