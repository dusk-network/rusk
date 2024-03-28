<svelte:options immutable={true} />

<script>
  import {
    mdiApplicationCogOutline,
    mdiArrowLeft,
    mdiCheckNetworkOutline,
    mdiGasStationOutline,
    mdiRestoreAlert,
  } from "@mdi/js";
  import { mapWith, rename } from "lamb";

  import {
    Badge,
    Button,
    ErrorDetails,
    Icon,
    Select,
    Switch,
  } from "$lib/dusk/components";
  import { AppAnchorButton, GasControls } from "$lib/components";
  import { currencies } from "$lib/dusk/currency";
  import { gasStore, settingsStore, walletStore } from "$lib/stores";
  import { areValidGasSettings } from "$lib/contracts";
  import { logout } from "$lib/navigation";
  import loginInfoStorage from "$lib/services/loginInfoStorage";

  const confirmResetMessage =
    "Confirm you've saved your recovery phrase before resetting the wallet. Proceed?";

  const resetWallet = () =>
    walletStore
      .clearLocalData()
      .then(() => {
        loginInfoStorage.remove();
        settingsStore.reset();
        logout(false);
      })
      .catch((err) => {
        resetError = err;
      });

  function handleResetWalletClick() {
    // eslint-disable-next-line no-alert
    if (confirm(confirmResetMessage)) {
      resetError = null;
      resetWallet();
    }
  }

  /** @type {(currency: { code: string, currency: string }) => SelectOption} */
  const currencyToOption = rename({ code: "value", currency: "label" });
  const currenciesToOptions = mapWith(currencyToOption);
  const { currency, darkMode, gasLimit, gasPrice, network } = $settingsStore;
  const { gasLimitLower, gasLimitUpper, gasPriceLower } = $gasStore;
  const networks = [
    { label: "testnet", value: "testnet" },
    { disabled: true, label: "mainnet", value: "mainnet" },
  ];

  let isDarkMode = darkMode;
  let isGasValid = false;

  /** @type {Error | null} */
  let resetError = null;

  $: ({ isSyncing } = $walletStore);
</script>

<section class="settings">
  <header class="settings__header">
    <h2>Settings</h2>
  </header>

  <div class="settings__content">
    <hr />
    <article class="settings-group">
      <header class="settings-group__header settings-group__header--network">
        <div class="settings-group__header">
          <Icon path={mdiCheckNetworkOutline} />
          <h3 class="h4 settings-group__heading">Network</h3>
        </div>
        <Badge variant="success" text="Online" />
      </header>
      <Select
        className="settings-group__select"
        value={network}
        on:change={(evt) => {
          settingsStore.update((store) => {
            // eslint-disable-next-line no-extra-parens
            const option = /** @type {HTMLInputElement} */ (evt.target);

            store.network = option.value;

            return store;
          });
        }}
        options={networks}
      />
    </article>
    <hr />
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiGasStationOutline} />
        <h3 class="h4 settings-group__heading">Gas</h3>
      </header>
      <div class="settings-group__multi-control-content">
        <GasControls
          on:gasSettings={(event) => {
            isGasValid = areValidGasSettings(
              event.detail.price,
              event.detail.limit
            );

            if (isGasValid) {
              settingsStore.update((store) => {
                store.gasLimit = event.detail.limit;
                store.gasPrice = event.detail.price;

                return store;
              });
            }
          }}
          limit={gasLimit}
          limitLower={gasLimitLower}
          limitUpper={gasLimitUpper}
          price={gasPrice}
          priceLower={gasPriceLower}
        />
      </div>
    </article>
    <hr />
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiApplicationCogOutline} />
        <h3 class="h4 settings-group__heading">Preferences</h3>
      </header>
      <div class="settings-group__multi-control-content">
        <label
          class="settings-group__control settings-group__control--switch"
          for={undefined}
        >
          <span>Dark mode</span>
          <Switch
            bind:value={isDarkMode}
            on:change={() => {
              settingsStore.update((store) => {
                store.darkMode = isDarkMode;

                return store;
              });
            }}
          />
        </label>
        <label
          class="settings-group__control settings-group__control--with-label"
          for={undefined}
        >
          <span>Currency</span>
          <Select
            className="settings-group__control settings-group__control--with-label"
            value={currency}
            on:change={(evt) => {
              settingsStore.update((store) => {
                // eslint-disable-next-line no-extra-parens
                const option = /** @type {HTMLInputElement} */ (evt.target);

                store.currency = option.value;

                return store;
              });
            }}
            options={currenciesToOptions(currencies)}
          />
        </label>
      </div>
    </article>
    <hr />
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiRestoreAlert} />
        <h3 class="h4 settings-group__heading">Danger zone</h3>
      </header>
      <ErrorDetails
        error={resetError}
        summary="An error occurred while resetting the wallet. Please try again."
      />
      <Button
        className="settings-group__button--state--danger"
        disabled={isSyncing}
        data-tooltip-disabled={!isSyncing}
        data-tooltip-id="main-tooltip"
        data-tooltip-text="Not allowed to reset while syncing"
        data-tooltip-type="warning"
        on:click={handleResetWalletClick}
        text="Reset Wallet"
      />
    </article>
  </div>
</section>

<div class="settings-actions">
  <AppAnchorButton
    href="/dashboard"
    disabled={!isGasValid}
    variant="tertiary"
    icon={{ path: mdiArrowLeft }}
    text="Back"
  />
  <Button on:click={() => logout(false)} variant="tertiary" text="Log out" />
</div>

<style lang="postcss">
  .settings {
    overflow-y: hidden;
    background-color: var(--surface-color);
    border-radius: 1.125em;

    & > * {
      padding: 1em 1em 0 1em;
    }

    &,
    &__content {
      display: flex;
      flex-direction: column;
      gap: var(--default-gap);
    }

    &__content {
      overflow-y: auto;
    }

    :global(& button, & select, & a) {
      width: 100%;
    }
  }

  .settings-group {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--small-gap);
    width: 100%;
    margin-bottom: 0.5em;

    &__header {
      display: flex;
      align-items: center;
      gap: 0.75em;

      &--network {
        width: 100%;
        justify-content: space-between;
      }
    }

    &__heading {
      line-height: 140%;
    }

    &__control {
      align-items: center;
      display: flex;
      flex-direction: row;
      justify-content: space-between;
      gap: 0.5em;
      width: 100%;

      &--with-label {
        flex-direction: column;
        justify-content: start;
        align-items: stretch;

        & > span {
          line-height: 140%;
        }
      }

      &--switch {
        background-color: var(--background-color);
        padding: 0.625em 0.75em 0.625em 0.75em;
        border-radius: 1.5em;
      }
    }

    &__multi-control-content {
      display: flex;
      flex-direction: column;
      gap: var(--default-gap);
      width: 100%;
    }

    :global(&__select) {
      text-transform: uppercase;
    }

    &:last-of-type {
      margin-bottom: 1em;
    }
  }

  .settings-actions {
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    gap: var(--default-gap);

    :global(& button, & a) {
      width: 100%;
    }
  }

  :global(.dusk-button.settings-group__button--state--danger) {
    background-color: var(--danger-color);
    color: var(--on-danger-color);
  }
</style>
