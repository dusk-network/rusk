<script>
  import { Button, ErrorDetails, Throbber } from "$lib/dusk/components";
  import { AppImage, Banner } from "$lib/components";
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
    <div class="welcome-layout__error-container">
      <Banner
        className="welcome-layout__error"
        title="Network Connection Issue"
        variant="error"
      >
        <p>
          The Web Wallet is currently unable to connect to the network. Please
          click "Retry" to attempt connecting again or check back in a few
          minutes.
        </p>
        <ErrorDetails {error} summary="Error details" />
      </Banner>
      <Button
        className="welcome-layout__retry-button"
        text="Retry"
        on:click={handleRetry}
      />
    </div>
  {/await}
{/key}

<style lang="postcss">
  .welcome-layout__error-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--medium-gap);
    width: 100%;
  }
  :global {
    .welcome-layout__loading {
      width: 100%;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--medium-gap);
    }

    .welcome-layout__error > .banner__content {
      gap: var(--medium-gap);
    }

    .welcome-layout__retry-button {
      width: 100%;
    }
  }
</style>
