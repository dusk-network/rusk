<script>
  /**
   * Note: shieldedTokensPercentage props in `` is passed the same balance twice.
   * This is because we currently don't have the separate balances
   * for phoenix and moonlight.
   */
  import {
    mdiAlertOutline,
    mdiCogOutline,
    mdiLink,
    mdiRestore,
    mdiTimerSand,
  } from "@mdi/js";
  import { Button, Icon } from "$lib/dusk/components";
  import { settingsStore, walletStore } from "$lib/stores";
  import {
    AddressPicker,
    AppAnchorButton,
    Balance,
    SyncBar,
  } from "$lib/components";
  import { luxToDusk } from "$lib/dusk/currency";

  /** @type {import('./$types').LayoutData} */
  export let data;

  /** @type {string} */
  let syncStatusLabel = "";

  /** @type {string} */
  let networkStatusIconPath = "";

  /** @type {string} */
  let iconVariant = "";

  /** @type {number | undefined} */
  let fiatPrice;

  data.currentPrice.then((prices) => {
    fiatPrice = prices[currency.toLowerCase()];
  });

  const { currency, language } = $settingsStore;

  $: ({ network } = $settingsStore);
  $: ({ balance, currentProfile, profiles, syncStatus } = $walletStore);
  $: if (syncStatus.isInProgress) {
    iconVariant = "warning";
    networkStatusIconPath = mdiTimerSand;
    syncStatusLabel = `Dusk ${network}`;
  } else if (syncStatus.error) {
    iconVariant = "error";
    networkStatusIconPath = mdiAlertOutline;
    syncStatusLabel = `Dusk ${network} - Sync Failed`;
  } else {
    iconVariant = "success";
    networkStatusIconPath = mdiLink;
    syncStatusLabel = `Dusk ${network}`;
  }
</script>

<svelte:window
  on:beforeunload={(event) => {
    event.preventDefault();
  }}
/>

<section class="dashboard">
  <div class="dashboard-content">
    <h2 class="sr-only">Dashboard</h2>

    <AddressPicker {currentProfile} {profiles} />

    <Balance
      fiatCurrency={currency}
      {fiatPrice}
      locale={language}
      tokenCurrency="DUSK"
      tokens={luxToDusk(balance.shielded.value)}
      shieldedTokensPercentage={import.meta.env.VITE_FEATURE_ALLOCATE || false
        ? 100
        : undefined}
    />

    <slot />
  </div>
  <footer class="footer">
    <nav class="footer__navigation">
      <div class="footer__network-status">
        <Icon
          className="footer__network-status-icon footer__network-status-icon--{iconVariant}"
          data-tooltip-disabled={!syncStatus.error}
          data-tooltip-id="main-tooltip"
          data-tooltip-text={syncStatus.error?.message}
          data-tooltip-type="error"
          path={networkStatusIconPath}
          size="large"
        />
        <div class="footer__network-message">
          {#if syncStatusLabel && !syncStatus.isInProgress}
            <span>{syncStatusLabel}</span>
          {/if}
          {#if syncStatus.isInProgress}
            <span>
              {syncStatusLabel} –
              <b
                >Syncing... {syncStatus.progress
                  ? `${syncStatus.progress * 100}%`
                  : ""}</b
              >
            </span>
            {#if syncStatus.progress}
              <SyncBar
                from={syncStatus.from}
                last={syncStatus.last}
                progress={syncStatus.progress}
              />
            {/if}
          {/if}
        </div>
      </div>
      <div class="footer__actions">
        {#if syncStatus.error}
          <Button
            aria-label="Retry synchronization"
            className="footer__actions-button footer__actions-button--retry"
            data-tooltip-id="main-tooltip"
            data-tooltip-text="Retry synchronization"
            icon={{ path: mdiRestore, size: "large" }}
            on:click={() => {
              walletStore.sync();
            }}
            variant="secondary"
          />
        {/if}
        <AppAnchorButton
          variant="secondary"
          className="footer__actions-button footer__actions-button--settings"
          icon={{ path: mdiCogOutline, size: "large" }}
          href="/settings"
          aria-label="Settings"
          data-tooltip-id="main-tooltip"
          data-tooltip-text="Settings"
        />
      </div>
    </nav>
  </footer>
</section>

<style lang="postcss">
  .dashboard {
    position: relative;
    display: flex;
    align-content: space-between;
    gap: 1.375rem;
    flex: 1;
    flex-direction: column;
    max-height: 100%;
  }

  .dashboard-content {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.375rem;
    overflow-y: auto;
    flex: 1;
  }

  .footer {
    width: 100%;

    &__navigation {
      display: flex;
      justify-content: space-between;
      gap: 0.75rem;
      align-items: center;
      width: 100%;
    }

    &__actions {
      display: flex;
      flex-direction: row;
      gap: 0.75em;
      align-items: center;
    }

    &__network-status {
      display: flex;
      align-items: center;
      gap: var(--small-gap);
      line-height: 150%;
      text-transform: capitalize;
      width: 100%;
    }

    &__network-message {
      display: flex;
      flex-direction: column;
      align-items: flex-start;
      width: 100%;
    }

    :global(.footer__network-status-icon) {
      border-radius: 50%;
      padding: 0.2em;
    }

    :global(.footer__network-status-icon--error) {
      cursor: help;
      color: var(--on-error-color);
      background: var(--error-color);
    }

    :global(.footer__network-status-icon--success) {
      color: var(--on-success-color);
      background: var(--success-color);
    }

    :global(.footer__network-status-icon--warning) {
      color: var(--on-warning-color);
      background: var(--warning-color);
    }
  }
</style>
