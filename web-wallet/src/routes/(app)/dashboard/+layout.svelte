<script>
  import { afterNavigate } from "$app/navigation";
  import {
    mdiAlertOutline,
    mdiCogOutline,
    mdiLink,
    mdiRestore,
    mdiTimerSand,
  } from "@mdi/js";
  import { Button, Icon } from "$lib/dusk/components";
  import { networkStore, settingsStore, walletStore } from "$lib/stores";
  import {
    AddressPicker,
    AppAnchorButton,
    Balance,
    SyncBar,
  } from "$lib/components";

  /** @type {import('./$types').LayoutData} */
  export let data;

  /** @type {HTMLDivElement} */
  let scrollContainer;

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
  const { networkName } = $networkStore;

  const scrollToTop = () => {
    scrollContainer.scrollTo(0, 0);
  };

  afterNavigate(scrollToTop);

  $: ({ balance, currentProfile, profiles, syncStatus } = $walletStore);
  $: if (syncStatus.isInProgress) {
    iconVariant = "warning";
    networkStatusIconPath = mdiTimerSand;
    syncStatusLabel = `Dusk ${networkName}`;
  } else if (syncStatus.error) {
    iconVariant = "error";
    networkStatusIconPath = mdiAlertOutline;
    syncStatusLabel = `Dusk ${networkName} - Sync Failed`;
  } else {
    iconVariant = "success";
    networkStatusIconPath = mdiLink;
    syncStatusLabel = `Dusk ${networkName}`;
  }

  /**
   * @param {Profile} profile
   */
  function setCurrentProfile(profile) {
    walletStore.setCurrentProfile(profile);
  }
</script>

<svelte:window
  on:beforeunload={(event) => {
    event.preventDefault();
  }}
/>

<section class="dashboard">
  <div
    class="dashboard-content"
    bind:this={scrollContainer}
    on:wizardstepchange={scrollToTop}
  >
    <h2 class="sr-only">Dashboard</h2>

    <AddressPicker
      {currentProfile}
      {profiles}
      on:setCurrentProfile={(event) => setCurrentProfile(event.detail.profile)}
    />

    <Balance
      fiatCurrency={currency}
      {fiatPrice}
      locale={language}
      tokenCurrency="DUSK"
      shieldedAmount={balance.shielded.value}
      unshieldedAmount={balance.unshielded.value}
    />

    <slot />
  </div>
  <footer class="footer">
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
                ? `${(syncStatus.progress * 100).toFixed(0)}%`
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
  </footer>
</section>

<style lang="postcss">
  :global {
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
      display: flex;
      justify-content: space-between;
      gap: 0.75rem;
      align-items: center;

      &__actions {
        display: flex;
        flex-direction: row;
        gap: 0.75em;
        align-items: center;
      }

      &__network-status {
        display: flex;
        height: 100%;
        align-items: center;
        gap: var(--small-gap);
        line-height: 150%;
        text-transform: capitalize;
        flex: 1;
      }

      &__network-message {
        display: flex;
        flex-direction: column;
        align-items: flex-start;
        flex: 1;
      }

      .footer__network-status-icon {
        border-radius: 50%;
        padding: 0.2em;
      }

      .footer__network-status-icon--error {
        cursor: help;
        color: var(--on-error-color);
        background: var(--error-color);
      }

      .footer__network-status-icon--success {
        color: var(--on-success-color);
        background: var(--success-color);
      }

      .footer__network-status-icon--warning {
        color: var(--on-warning-color);
        background: var(--warning-color);
      }
    }
  }
</style>
