<svelte:options immutable={true} />

<script>
  import {
    mdiAccountQuestionOutline,
    mdiApplicationCogOutline,
    mdiArrowLeft,
    mdiGasStationOutline,
    mdiRestoreAlert,
  } from "@mdi/js";
  import { mapWith, rename } from "lamb";
  import {
    Anchor,
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

  const confirmResetGasMessage =
    "Are you sure you want to reset the gas settings to their defaults?";
  const confirmResetWalletMessage =
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

  function handleResetGasSettingsClick() {
    // eslint-disable-next-line no-alert
    if (confirm(confirmResetGasMessage)) {
      settingsStore.resetGasSettings();
    }
  }

  function handleResetWalletClick() {
    // eslint-disable-next-line no-alert
    if (confirm(confirmResetWalletMessage)) {
      resetError = null;
      resetWallet();
    }
  }

  /** @type {(currency: { code: string, currency: string }) => SelectOption} */
  const currencyToOption = rename({ code: "value", currency: "label" });
  const currenciesToOptions = mapWith(currencyToOption);
  const { gasLimitLower, gasLimitUpper, gasPriceLower } = $gasStore;

  let isGasValid = false;

  /** @type {Error | null} */
  let resetError = null;

  $: ({ syncStatus } = $walletStore);
  $: ({ currency, darkMode, gasLimit, gasPrice } = $settingsStore);
</script>

<section class="settings">
  <header class="settings__header">
    <h2>Settings</h2>
  </header>

  <div class="settings__content">
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiGasStationOutline} />
        <h3 class="h4 settings-group__heading">Gas</h3>
      </header>
      <div class="settings-group__multi-control-content">
        <GasControls
          on:gasSettings={(event) => {
            const { limit, price } = event.detail;

            isGasValid = areValidGasSettings(price, limit);

            if (isGasValid) {
              settingsStore.update((store) => ({
                ...store,
                gasLimit: limit,
                gasPrice: price,
              }));
            }
          }}
          limit={gasLimit}
          limitLower={gasLimitLower}
          limitUpper={gasLimitUpper}
          price={gasPrice}
          priceLower={gasPriceLower}
        />
        <Button
          on:click={handleResetGasSettingsClick}
          text="Reset to defaults"
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
          class="settings-group__control settings-group__control--with-label"
          for={undefined}
        >
          <span>Currency:</span>
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
        <label
          class="settings-group__control settings-group__control--switch"
          for={undefined}
        >
          <span>Dark mode</span>
          <Switch
            on:change={(event) => {
              settingsStore.update((store) => {
                store.darkMode = event.detail;

                return store;
              });
            }}
            value={darkMode}
          />
        </label>
      </div>
    </article>
    <hr />
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiAccountQuestionOutline} />
        <h3 class="h4 settings-group__heading">Support</h3>
      </header>
      <p>
        Need help or have feedback? Explore the <Anchor
          rel="noopener noreferrer"
          target="_blank"
          href="https://docs.dusk.network">Dusk Docs</Anchor
        >
        for detailed documentation, join the
        <Anchor
          rel="noopener noreferrer"
          target="_blank"
          href="https://discord.gg/dusk-official">community Discord</Anchor
        > for questions and discussions, and visit the
        <Anchor
          rel="noopener noreferrer"
          target="_blank"
          href="https://github.com/dusk-network/rusk">GitHub repository</Anchor
        > to view known issues, report bugs, or share feedback.
      </p>
    </article>
    <hr />
    <article class="settings-group">
      <header class="settings-group__header">
        <Icon path={mdiRestoreAlert} />
        <h3 class="h4 settings-group__heading">Reset wallet</h3>
      </header>
      <div class="settings-group__multi-control-content">
        <p>
          Resetting your wallet clears the cache and restores default settings
          without affecting your funds or transaction history. You’ll need your
          mnemonic phrase to restore access, so ensure it’s securely stored
          before proceeding.
        </p>
        <ErrorDetails
          error={resetError}
          summary="An error occurred while resetting the wallet. Please try again."
        />
        <Button
          className="settings-group__button--state--danger"
          disabled={syncStatus.isInProgress}
          data-tooltip-disabled={!syncStatus.isInProgress}
          data-tooltip-id="main-tooltip"
          data-tooltip-text="Not allowed to reset while syncing"
          data-tooltip-type="warning"
          on:click={handleResetWalletClick}
          text="Reset Wallet"
        />
      </div>
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
      text-transform: capitalize;
      display: flex;
      align-items: center;
      gap: 0.75em;
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
    background-color: var(--error-color);
    color: var(--on-danger-color);
  }
</style>
