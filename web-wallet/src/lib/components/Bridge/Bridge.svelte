<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { mdiArrowUpBoldBoxOutline, mdiHistory } from "@mdi/js";
  import { parseEther, parseUnits } from "viem";
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
    Banner,
    GasFee,
    GasSettings,
    OperationResult,
  } from "$lib/components";
  import { account, duskEvm, wagmiConfig } from "$lib/web3/walletConnection";
  import bridgeABI from "$lib/web3/abi/bridgeABI.json";
  import { logo } from "$lib/dusk/icons";
  import { MESSAGES } from "$lib/constants";
  import { luxToDusk } from "$lib/dusk/currency";
  import { getDecimalSeparator, slashDecimals } from "$lib/dusk/number";
  import { cleanNumberString } from "$lib/dusk/string";
  import { areValidGasSettings } from "$lib/contracts";
  import { walletStore } from "$lib/stores";
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

  /** @type {string} */
  let destinationNetwork = "";

  /** @type {string} */
  let amount = "";

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
        value: parseEther(amount),
      });
    } else if (originNetwork === "duskDs" && destinationNetwork === "duskEvm") {
      // Deposit...

      const gas = new Gas({ limit: gasLimit, price: gasPrice });

      if (!$account.address) {
        throw new Error("Account address is not available.");
      }

      const response = await walletStore.depositEvmFunctionCall(
        $account.address,
        parseUnits(amount, 9),
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
  $: isNextButtonDisabled = amount === "" || (isDepositing && !isGasValid);
  $: isGasValid = areValidGasSettings(gasPrice, gasLimit);
  $: ({ address } = $account);
</script>

<article class="bridge">
  <header class="bridge__header">
    <h3 class="h4">Bridge</h3>
    <div class="bridge__header-icons">
      <AppAnchor href="/dashboard/bridge/transactions">
        <Icon path={mdiHistory} />
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
                {`${formatter(luxToDusk(parseUnits(amount, 9)))} DUSK`}
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
          successMessage="Transaction pending"
        >
          <svelte:fragment slot="success-content" let:result={hash}>
            <p>{MESSAGES.TRANSACTION_PENDING}</p>
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
