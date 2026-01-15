<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { mdiArrowUpBoldBoxOutline, mdiHistory } from "@mdi/js";
  import { parseUnits } from "viem";
  import { switchChain, writeContract } from "@wagmi/core";
  import { getKey } from "lamb";
  import { Gas } from "@dusk/w3sper";
  import { bytesToHexString } from "@duskit/encoding";

  import {
    AnchorButton,
    Badge,
    Icon,
    Select,
    Textbox,
    Throbber,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import {
    AppAnchor,
    AppAnchorButton,
    Banner,
    GasFee,
    GasSettings,
    OperationResult,
  } from "$lib/components";
  import { account, duskEvm, wagmiConfig } from "$lib/web3/walletConnection";
  import bridgeABI from "$lib/web3/abi/bridgeABI.json";
  import { logo } from "$lib/dusk/icons";
  import { formatBlocksAsTime } from "$lib/bridge/formatBlocksAsTime";
  import { countPendingWithdrawalsFor } from "$lib/bridge/pendingWithdrawals";
  import { MESSAGES } from "$lib/constants";
  import { luxToDusk } from "$lib/dusk/currency";
  import {
    createNumberFormatter,
    getDecimalSeparator,
    slashDecimals,
  } from "$lib/dusk/number";
  import { cleanNumberString } from "$lib/dusk/string";
  import { areValidGasSettings } from "$lib/contracts";
  import { settingsStore, walletStore } from "$lib/stores";
  import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

  /** @type {string} */
  const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;

  /** @type {`0x${string}`} */
  const VITE_EVM_BRIDGE_CONTRACT_ADDRESS = import.meta.env
    .VITE_EVM_BRIDGE_CONTRACT_ADDRESS;

  /**
   * The value needs to be passed but is ignored by the EVM, therefore we're setting it to zero.
   * @type {number}
   */
  const EVM_MINIMUM_GAS_LIMIT = 0;

  /**
   * DuskEVM uses duskEvm.nativeCurrency.decimals (currently 18). DuskDS uses 9 decimals (Lux).
   * This converts from EVM base units (1e18) to Lux units (1e9) so we can reuse luxToDusk().
   * @type {bigint}
   */
  const EVM_TO_LUX_SCALE_FACTOR =
    10n ** BigInt(duskEvm.nativeCurrency.decimals - 9);

  /** @type {(amount: bigint|number) => string} */
  export let formatter;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {string} */
  export let unshieldedAddress;

  /** @type {bigint} */
  export let unshieldedBalance;

  /** @type {import('@wagmi/core').GetBalanceReturnType | undefined} */
  export let evmDuskBalance;

  /** @type {Promise<PendingWithdrawalEntry[]>} */
  export let pendingWithdrawals = Promise.resolve([]);

  /** @type {string} */
  let language = "en";

  /** @type {(n: number | bigint) => string} */
  let blocksFormatter = (n) => `${n}`;

  /** @type {bigint | null} */
  let finalizationPeriodBlocks = null;

  /** @type {boolean} */
  let isFinalizationPeriodLoading = false;

  async function loadFinalizationPeriod() {
    if (finalizationPeriodBlocks !== null || isFinalizationPeriodLoading) {
      return;
    }

    isFinalizationPeriodLoading = true;

    try {
      const contract = await walletStore.useContract(
        VITE_BRIDGE_CONTRACT_ID,
        wasmPath
      );
      const period = await contract.call.finalization_period();
      finalizationPeriodBlocks =
        typeof period === "bigint" ? period : BigInt(period);
    } catch {
      finalizationPeriodBlocks = null;
    } finally {
      isFinalizationPeriodLoading = false;
    }
  }

  /** @type {string} */
  let destinationNetwork = "";

  /** @type {string} */
  let amount = "";

  /** @type {bigint} */
  let amountLux = 0n;

  /** @type {bigint} */
  let amountWei = 0n;

  /** @type {boolean} */
  let isGasValid = true;

  /** @type {ContractGasSettings} */
  let { gasLimit, gasPrice } = gasSettings;

  /**
   * Determines the direction of the transaction as either a withdrawal or deposit,
   * makes the appropriate contract call and returns the transaction hash.
   *
   * @return {Promise<string | undefined>} hash
   */
  async function bridge() {
    let hash;

    if (originNetwork === "duskEvm" && destinationNetwork === "duskDs") {
      // Withdraw...

      /** @type {[number,string] | []} */
      let args = [];

      await walletStore
        .useContract(VITE_BRIDGE_CONTRACT_ID, wasmPath)
        .then(async (contract) => {
          const encodedExtraData = await contract.encode(
            "extra_data",
            unshieldedAddress
          );
          args = [
            EVM_MINIMUM_GAS_LIMIT,
            `0x${bytesToHexString(encodedExtraData)}`,
          ];
        });

      await switchChain(wagmiConfig, { chainId: duskEvm.id });

      hash = await writeContract(wagmiConfig, {
        abi: bridgeABI,
        address: VITE_EVM_BRIDGE_CONTRACT_ADDRESS,
        args,
        chainId: duskEvm.id,
        functionName: "bridgeETH",
        value: amountWei,
      });
    } else if (originNetwork === "duskDs" && destinationNetwork === "duskEvm") {
      // Deposit...

      const gas = new Gas({ limit: gasLimit, price: gasPrice });

      if (!$account.address) {
        throw new Error("Account address is not available.");
      }

      const response = await walletStore.depositEvmFunctionCall(
        $account.address,
        amountLux,
        VITE_BRIDGE_CONTRACT_ID,
        wasmPath,
        gas
      );

      hash = getKey("hash")(response);
    } else {
      throw new Error("Invalid bridge operation.");
    }

    return hash;
  }

  /**
   * Options used for selecting which network to bridge between in the UI.
   *
   * @type {Array<SelectOption>}
   */
  const baseOptions = [
    { label: "DuskDS", value: "duskDs" },
    { label: "DuskEVM", value: "duskEvm" },
  ];

  // Default origin
  let originNetwork = baseOptions[0].value;

  $: destinationNetworkOptions = baseOptions.map((option) => ({
    ...option,
    disabled: option.value === originNetwork,
  }));
  $: destinationNetwork =
    destinationNetworkOptions.find((o) => !o.disabled)?.value ?? "";
  $: amount = slashDecimals(cleanNumberString(amount, getDecimalSeparator()));
  $: fee = gasLimit * gasPrice;
  $: isDepositing =
    originNetwork === "duskDs" && destinationNetwork === "duskEvm";
  $: isWithdrawing =
    originNetwork === "duskEvm" && destinationNetwork === "duskDs";
  $: if (isWithdrawing) {
    loadFinalizationPeriod();
  }
  $: {
    // viem expects a dot as decimal separator.
    // We also guard against incomplete inputs like "1." or ",5".
    const normalized = amount.replace(",", ".");
    const trimmed = normalized.endsWith(".")
      ? normalized.slice(0, -1)
      : normalized;
    const safe = trimmed.startsWith(".") ? `0${trimmed}` : trimmed;

    try {
      amountLux = safe ? parseUnits(safe, 9) : 0n;
    } catch {
      amountLux = 0n;
    }

    // Convert Lux (1e9) to EVM base units (1e18)
    amountWei = amountLux * EVM_TO_LUX_SCALE_FACTOR;
  }
  $: totalDepositLux = amountLux + fee;
  $: hasEnoughUnshielded =
    !isDepositing || unshieldedBalance >= totalDepositLux;
  $: hasEnoughEvm =
    !isWithdrawing ||
    (evmDuskBalance ? amountWei <= evmDuskBalance.value : false);
  $: isBalanceSufficient = isDepositing ? hasEnoughUnshielded : hasEnoughEvm;
  $: isNextButtonDisabled =
    amountLux === 0n || (isDepositing && !isGasValid) || !isBalanceSufficient;
  $: isGasValid = areValidGasSettings(gasPrice, gasLimit);
  $: ({ address } = $account);
  $: ({ language } = $settingsStore);
  $: blocksFormatter = createNumberFormatter(language, 0);
</script>

<article class="bridge">
  <header class="bridge__header">
    <h3 class="h4">Bridge</h3>
    <div class="bridge__header-icons">
      <AppAnchor
        href="/dashboard/bridge/transactions"
        className="bridge__transactions-link"
        aria-label="Pending withdrawals"
      >
        <Icon path={mdiHistory} />
        {#await pendingWithdrawals then withdrawals}
          {@const pendingCount = countPendingWithdrawalsFor(
            unshieldedAddress,
            withdrawals
          )}
          {#if pendingCount > 0}
            <span class="bridge__transactions-indicator" aria-hidden="true">
              {pendingCount > 9 ? "9+" : pendingCount}
            </span>
          {/if}
        {/await}
      </AppAnchor>
    </div>
  </header>

  <aside class="bridge__balances">
    <dl class="balances">
      <dt class="balances__token">DuskDS</dt>
      <dd class="balances__balance">
        {formatter(luxToDusk(unshieldedBalance))}
      </dd>
      <dt class="balances__token">DuskEVM</dt>
      <dd class="balances__balance">
        {#if evmDuskBalance}
          {formatter(luxToDusk(evmDuskBalance.value / EVM_TO_LUX_SCALE_FACTOR))}
        {:else}
          <Throbber size={16} />
        {/if}
      </dd>
    </dl>
  </aside>

  <div class="operation">
    <Wizard steps={3} let:key>
      <WizardStep
        step={0}
        {key}
        backButton={{
          disabled: false,
          href: "/dashboard",
          isAnchor: true,
        }}
        nextButton={{ disabled: isNextButtonDisabled }}
      >
        <div in:fade|global class="operation__bridge">
          <fieldset class="operation__fieldset">
            <p class="operation__label">From</p>

            <div class="operation__input-wrapper">
              <Select
                bind:value={originNetwork}
                name="origin-network"
                options={baseOptions}
              />
            </div>

            <hr class="glyph" />

            <p class="operation__label">To</p>

            <div class="operation__input-wrapper">
              <Select
                bind:value={destinationNetwork}
                name="destination-network"
                options={destinationNetworkOptions}
              />
            </div>

            <div class="operation__input-wrapper">
              <Textbox
                className="operation__input-field"
                required
                type="text"
                id="destination-amount"
                name="destination-amount"
                bind:value={amount}
              />
              <Icon
                data-tooltip-id="main-tooltip"
                data-tooltip-text="DUSK"
                path={logo}
              />
            </div>

            {#if amount !== ""}
              {#if isDepositing && !hasEnoughUnshielded}
                <Banner title="Insufficient balance" variant="warning">
                  <p>
                    Your <b>unshielded</b> balance must cover the amount
                    <i>plus</i> the fee.
                  </p>
                </Banner>
              {:else if isWithdrawing && !evmDuskBalance}
                <Banner title="Checking balance" variant="info">
                  <p>Fetching your <b>DuskEVM</b> balance…</p>
                </Banner>
              {:else if isWithdrawing && evmDuskBalance && !hasEnoughEvm}
                <Banner title="Insufficient balance" variant="warning">
                  <p>
                    Your <b>DuskEVM</b> balance is too low for this withdrawal (you
                    also need a little extra for gas).
                  </p>
                </Banner>
              {/if}
            {/if}
          </fieldset>
          {#if isDepositing}
            <GasSettings
              {formatter}
              {fee}
              limit={gasSettings.gasLimit}
              limitLower={gasLimits.gasLimitLower}
              limitUpper={gasLimits.gasLimitUpper}
              price={gasSettings.gasPrice}
              priceLower={gasLimits.gasPriceLower}
              on:gasSettings={(event) => {
                gasPrice = event.detail.price;
                gasLimit = event.detail.limit;
              }}
            />
          {/if}
        </div>
      </WizardStep>
      <WizardStep
        step={1}
        {key}
        nextButton={{
          disabled: isNextButtonDisabled,
          icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
          label: "SEND",
          variant: "primary",
        }}
      >
        <div in:fade|global class="operation__bridge">
          <Badge
            className="operation__review-notice"
            text="REVIEW TRANSACTION"
            variant="warning"
          />
          <dl class="operation__review-transaction">
            <dt class="review-transaction__label">
              <Icon path={mdiArrowUpBoldBoxOutline} />
              <span>Amount:</span>
            </dt>
            <dd class="review-transaction__value operation__review-amount">
              <span>
                {`${formatter(luxToDusk(amountLux))} DUSK`}
              </span>
              <Icon
                className="dusk-amount__icon"
                path={logo}
                data-tooltip-id="main-tooltip"
                data-tooltip-text="DUSK"
              />
            </dd>
          </dl>
          <dl class="operation__review-transaction">
            <dt class="review-transaction__label">
              <span>From</span>
            </dt>
            <dd class="operation__review-address">
              <span>
                {isDepositing ? unshieldedAddress : address}
              </span>
            </dd>
          </dl>
          <dl class="operation__review-transaction">
            <dt class="review-transaction__label">
              <span>To</span>
            </dt>
            <dd class="operation__review-address">
              <span>
                {isDepositing ? address : unshieldedAddress}
              </span>
            </dd>
          </dl>
          {#if isDepositing}
            <GasFee {formatter} {fee} />
          {/if}
          <Banner title="Fee Details" variant="info">
            <p>
              The fee will be deducted from your <b
                >{isDepositing ? "unshielded" : "EVM"}</b
              > balance.
            </p>
          </Banner>
        </div>
      </WizardStep>
      <WizardStep step={2} {key} showNavigation={false}>
        <OperationResult
          errorMessage="Bridging failed"
          operation={bridge()}
          pendingMessage="Processing transaction"
          successMessage={isDepositing
            ? "Deposit submitted"
            : "Withdrawal request submitted"}
        >
          <svelte:fragment slot="success-content" let:result={hash}>
            {#if isDepositing}
              <p>{MESSAGES.TRANSACTION_CREATED}</p>
            {:else}
              <p>{MESSAGES.TRANSACTION_PENDING}</p>

              {#if isFinalizationPeriodLoading}
                <p class="bridge__finalization-hint">
                  <Throbber size={16} /> Fetching finalization period…
                </p>
              {:else if finalizationPeriodBlocks !== null}
                <p class="bridge__finalization-hint">
                  Finalization period:
                  {blocksFormatter(finalizationPeriodBlocks)} blocks ({formatBlocksAsTime(
                    finalizationPeriodBlocks,
                    language
                  )} at ~10s/block). You can finalize your withdrawal once the period
                  has passed.
                </p>
              {/if}

              <AppAnchorButton
                href="/dashboard/bridge/transactions"
                text="PENDING WITHDRAWALS"
                variant="primary"
              />
            {/if}
            {#if hash}
              <AnchorButton
                href={isDepositing
                  ? `/explorer/transactions/transaction?id=${hash}`
                  : `${duskEvm.blockExplorers.default.url}/tx/${hash}`}
                text="VIEW ON BLOCK EXPLORER"
                rel="noopener noreferrer"
                target="_blank"
              />
            {/if}
          </svelte:fragment>
        </OperationResult>
      </WizardStep>
    </Wizard>
  </div>
</article>

<style lang="postcss">
  .bridge {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding: 1.25em;

    &__header {
      display: flex;
      justify-content: space-between;
    }

    &__header-icons {
      display: flex;
      align-items: center;
      gap: 0.675em;
    }

    :global(&__transactions-link) {
      position: relative;
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }

    :global(&__transactions-link .dusk-icon) {
      /* Make the transaction history icon a little bit bigger */
      --icon-size: 1.8rem;
    }

    &__transactions-indicator {
      position: absolute;
      top: -0.45em;
      right: -0.45em;

      min-width: 1.4em;
      height: 1.4em;
      padding: 0 0.4em;

      border-radius: 999px;
      background: var(--error-color);
      color: var(--on-error-color);

      font-size: 0.75em;
      font-weight: 700;
      line-height: 1.4em;

      display: inline-flex;
      align-items: center;
      justify-content: center;

      border: 2px solid var(--surface-color);
      box-sizing: border-box;
      pointer-events: none;
    }

    &__finalization-hint {
      opacity: 0.9;
      line-height: 1.4;
      margin-top: 0.75em;
    }

    &__balances {
      display: flex;
      padding: 1em 1.25em;
      flex-direction: column;
      justify-content: center;
      align-items: flex-start;
      gap: var(--medium-gap);
      align-self: stretch;

      border-radius: var(--fieldset-border-radius);
      background: var(--fieldset-background-color);
    }
  }

  .balances {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.25rem 1rem;
    align-items: baseline;
    margin: 0;
    width: 100%;

    &__token {
      font-weight: 500;
    }

    &__balance {
      margin: 0;
      overflow-wrap: anywhere;
      text-align: right;
    }
  }

  .operation {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;

    &__fieldset {
      display: flex;
      padding: 1em 1.25em;
      flex-direction: column;
      justify-content: center;
      align-items: flex-start;
      gap: var(--medium-gap);
      align-self: stretch;

      border-radius: var(--fieldset-border-radius);
      background: var(--fieldset-background-color);
    }

    &__input-wrapper {
      column-gap: var(--default-gap);
      display: flex;
      align-items: center;
      width: 100%;
    }

    &__review-address {
      background-color: transparent;
      border: 1px solid var(--primary-color);
      border-radius: 1.5em;
      padding: 0.75em 1em;
      width: 100%;
      line-break: anywhere;
    }

    &__review-transaction {
      display: flex;
      flex-direction: column;
      gap: 0.625em;
    }

    &__review-amount {
      justify-content: flex-start;
    }

    &__bridge {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }

    .review-transaction__label,
    .review-transaction__value {
      display: inline-flex;
      align-items: center;
      gap: var(--small-gap);
    }

    .review-transaction__value {
      font-weight: bold;
    }

    :global(&__review-notice) {
      text-align: center;
    }

    & > :global(.operation__operation-button) {
      width: 100%;
      text-align: left;
    }
  }

  .glyph {
    margin: var(--default-gap) 0;
    height: 1px;
  }

  .glyph:after {
    content: "↑↓";
    display: inline-block;
    position: relative;
    top: -1.2em;
    color: var(--divider-border-color);
    border: 1px solid var(--divider-border-color);
    border-radius: 2em;
    padding: 0.5em 1.25em;
    background-color: var(--divider-background-color);
  }
</style>
