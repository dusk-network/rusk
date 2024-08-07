<script>
  import {
    mdiAlertOutline,
    mdiCogOutline,
    mdiLink,
    mdiRestore,
    mdiTimerSand,
  } from "@mdi/js";
  import { Button, Icon, ProgressBar } from "$lib/dusk/components";
  import { settingsStore, walletStore } from "$lib/stores";
  import { AppAnchorButton } from "$lib/components";

  $: ({ network } = $settingsStore);
  $: ({ isSyncing, error } = $walletStore);

  /** @type {string} */
  let syncStatus = "";

  /** @type {string} */
  let networkStatusIconPath = "";

  /** @type {string} */
  let iconVariant = "";

  $: if (isSyncing) {
    iconVariant = "warning";
    networkStatusIconPath = mdiTimerSand;
    syncStatus = `Syncing with Dusk ${network}`;
  } else if (error) {
    iconVariant = "error";
    networkStatusIconPath = mdiAlertOutline;
    syncStatus = `Dusk ${network} - Sync Failed`;
  } else {
    iconVariant = "success";
    networkStatusIconPath = mdiLink;
    syncStatus = `Dusk ${network}`;
  }
</script>

<svelte:window
  on:beforeunload={(event) => {
    event.preventDefault();
  }}
/>

<section class="dashboard">
  <slot />
  <footer class="footer">
    <nav class="footer__navigation">
      <div class="footer__network-status">
        <Icon
          className="footer__network-status-icon footer__network-status-icon--{iconVariant}"
          data-tooltip-disabled={!error}
          data-tooltip-id="main-tooltip"
          data-tooltip-text={error?.message}
          data-tooltip-type="error"
          path={networkStatusIconPath}
          size="large"
        />
        <div class="footer__network-message">
          <span>{syncStatus}</span>
          {#if isSyncing}
            <ProgressBar />
          {/if}
        </div>
      </div>
      <div class="footer__actions">
        {#if error}
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
    }

    &__network-message {
      display: flex;
      flex-direction: column;
      align-items: center;
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
