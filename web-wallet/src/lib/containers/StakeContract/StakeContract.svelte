<svelte:options immutable={true} />

<script>
  import {
    collect,
    getKey,
    hasKeyValue,
    map,
    mapWith,
    pick,
    setKey,
    when,
  } from "lamb";
  import { mdiArrowLeft } from "@mdi/js";
  import { Gas } from "$lib/vendor/w3sper.js/src/mod";

  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { getLastTransactionHash } from "$lib/transactions";
  import {
    gasStore,
    operationsStore,
    settingsStore,
    walletStore,
  } from "$lib/stores";
  import {
    AppAnchorButton,
    ContractOperations,
    ContractStatusesList,
    Stake,
  } from "$lib/components";
  import { Suspense, Throbber } from "$lib/dusk/components";

  /** @type {ContractDescriptor} */
  export let descriptor;

  const gasLimits = $gasStore;

  /**
   * Temporary replacement for the old `walletStore.getStakeInfo`
   * function.
   * The UI needs to be updated to just use the `stakeInfo` property
   * directly.
   */
  const getStakeInfo = async () => $walletStore.stakeInfo;

  const collectSettings = collect([
    pick([
      "gasLimit",
      "gasLimitLower",
      "gasLimitUpper",
      "gasPrice",
      "gasPriceLower",
    ]),
    getKey("language"),
    getKey("minAllowedStake"),
  ]);

  /** @type {Record<string, (info: StakeInfo) => boolean>} */
  const disablingConditions = {
    stake: (info) => !!info.amount,
    unstake: (info) => !info.amount || info.amount.total === 0n,
    "withdraw-rewards": (info) => info.reward <= 0n,
  };

  /** @type {Record<StakeType, (...args: any[]) => Promise<string>>} */
  const executeOperations = {
    stake: (amount, gasPrice, gasLimit) =>
      walletStore
        .stake(amount, new Gas({ limit: gasLimit, price: gasPrice }))
        .then(getLastTransactionHash),
    unstake: (gasPrice, gasLimit) =>
      walletStore
        .unstake(new Gas({ limit: gasLimit, price: gasPrice }))
        .then(getLastTransactionHash),
    "withdraw-rewards": (gasPrice, gasLimit) =>
      walletStore
        .withdrawReward(
          $walletStore.stakeInfo.reward,
          new Gas({ limit: gasLimit, price: gasPrice })
        )
        .then(getLastTransactionHash),
  };

  /** @type {(operations: ContractOperation[]) => ContractOperation[]} */
  const disableAllOperations = mapWith(setKey("disabled", true));

  /** @type {(operationId: string) => operationId is StakeType} */
  const isStakeOperation = (operationId) =>
    ["stake", "unstake", "withdraw-rewards"].includes(operationId);

  /**
   * We want to update the disabled status ourselves only
   * when the operation is enabled in the descriptor;
   * otherwise the descriptor takes precedence.
   *
   * @param {ContractOperation[]} operations
   * @param {StakeInfo} stakeInfo
   * @returns {ContractOperation[]}
   */
  const getOperations = (operations, stakeInfo) =>
    map(
      operations,
      when(
        hasKeyValue("disabled", false),
        updateOperationDisabledStatus(stakeInfo)
      )
    );

  /**
   * @param {StakeInfo} stakeInfo
   * @param {bigint} spendable
   * @returns {ContractStatus[]}
   */
  const getStatuses = (stakeInfo, spendable) => [
    {
      label: "Spendable",
      value: duskFormatter(luxToDusk(spendable)),
    },
    {
      label: "Total Locked",
      value: stakeInfo.amount
        ? duskFormatter(luxToDusk(stakeInfo.amount.locked))
        : "N/A",
    },
    {
      label: "Rewards",
      value: duskFormatter(luxToDusk(stakeInfo.reward)),
    },
  ];

  /**
   * @param {StakeInfo} stakeInfo
   * @returns {(operation: ContractOperation) => ContractOperation}
   */
  const updateOperationDisabledStatus = (stakeInfo) => (operation) => ({
    ...operation,
    disabled: disablingConditions[operation.id]?.(stakeInfo) ?? true,
  });

  $: ({ currentOperation } = $operationsStore);
  $: [gasSettings, language, minAllowedStake] = collectSettings($settingsStore);
  const { hideStakingNotice } = $settingsStore;
  $: ({ balance, syncStatus } = $walletStore);
  $: isSyncOK = !(syncStatus.isInProgress || !!syncStatus.error);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

{#key currentOperation}
  <Suspense
    gap="medium"
    errorMessage="Failed to retrieve stake info"
    errorVariant="details"
    waitFor={getStakeInfo()}
  >
    <svelte:fragment slot="pending-content">
      {#if !syncStatus.isInProgress && !syncStatus.error}
        <Throbber />
      {:else}
        <p>Data will load after a successful sync.</p>
      {/if}
    </svelte:fragment>
    <svelte:fragment slot="success-content" let:result={stakeInfo}>
      {@const statuses = getStatuses(stakeInfo, balance.shielded.spendable)}
      {@const operations = isSyncOK
        ? getOperations(descriptor.operations, stakeInfo)
        : disableAllOperations(descriptor.operations)}
      {#if isStakeOperation(currentOperation)}
        <Stake
          execute={executeOperations[currentOperation]}
          flow={currentOperation}
          formatter={duskFormatter}
          {gasLimits}
          {gasSettings}
          {minAllowedStake}
          on:operationChange
          on:suppressStakingNotice
          rewards={luxToDusk(stakeInfo.reward)}
          spendable={balance.shielded.spendable}
          staked={stakeInfo.amount ? luxToDusk(stakeInfo.amount.total) : 0}
          {statuses}
          {hideStakingNotice}
        />
      {:else}
        <ContractStatusesList items={statuses} />
        <ContractOperations items={operations} on:operationChange />
      {/if}
    </svelte:fragment>
    <svelte:fragment slot="error-actions">
      <AppAnchorButton
        className="error-action"
        href="/dashboard"
        icon={{ path: mdiArrowLeft }}
        text="Back"
        variant="tertiary"
      />
    </svelte:fragment>
  </Suspense>
{/key}

<style>
  :global(.error-action) {
    width: 100%;
    text-align: left;
  }
</style>
