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
  import { Gas } from "$lib/vendor/w3sper.js/src/mod";

  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import {
    gasStore,
    operationsStore,
    settingsStore,
    walletStore,
  } from "$lib/stores";
  import {
    ContractOperations,
    ContractStatusesList,
    Stake,
  } from "$lib/components";

  /** @type {ContractDescriptor} */
  export let descriptor;

  export let spendable = 0n;

  $: [gasSettings, language] = collectSettings($settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);

  const gasLimits = $gasStore;

  /** @type {ContractOperation[]} */
  let operations;

  $: stakeInfo = $walletStore.stakeInfo;

  $: if (stakeInfo) {
    operations = getOperations();
  }

  const collectSettings = collect([
    pick([
      "gasLimit",
      "gasLimitLower",
      "gasLimitUpper",
      "gasPrice",
      "gasPriceLower",
    ]),
    getKey("language"),
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
        .then(getKey("hash")),
    unstake: (gasPrice, gasLimit) =>
      walletStore
        .unstake(new Gas({ limit: gasLimit, price: gasPrice }))
        .then(getKey("hash")),
    "withdraw-rewards": (gasPrice, gasLimit) =>
      walletStore
        .withdrawReward(
          $walletStore.stakeInfo.reward,
          new Gas({ limit: gasLimit, price: gasPrice })
        )
        .then(getKey("hash")),
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
   * @returns {ContractOperation[]}
   */
  const getOperations = () =>
    map(
      descriptor.operations,
      when(hasKeyValue("disabled", false), updateOperationDisabledStatus())
    );

  /**
   * @returns {ContractStatus[]}
   */
  $: statuses = [
    {
      label: "Spendable",
      value: duskFormatter(luxToDusk(spendable)),
    },
    {
      label: "Active Stake",
      value: stakeInfo.amount
        ? duskFormatter(luxToDusk(stakeInfo.amount.value))
        : null,
    },
    {
      label: "Penalized Stake",
      value: stakeInfo.amount
        ? duskFormatter(luxToDusk(stakeInfo.amount.locked))
        : null,
    },
    {
      label: "Rewards",
      value: duskFormatter(luxToDusk(stakeInfo.reward)),
    },
  ];

  /**
   * @returns {(operation: ContractOperation) => ContractOperation}
   */
  const updateOperationDisabledStatus = () => (operation) => ({
    ...operation,
    disabled: disablingConditions[operation.id]?.(stakeInfo) ?? true,
  });

  $: ({ currentOperation } = $operationsStore);
  const { hideStakingNotice } = $settingsStore;
  $: ({ balance, minimumStake, syncStatus } = $walletStore);
  $: isSyncOK = !(syncStatus.isInProgress || !!syncStatus.error);
  $: if (!isSyncOK) {
    disableAllOperations(descriptor.operations);
  }
</script>

{#key currentOperation}
  {#if isStakeOperation(currentOperation)}
    <Stake
      execute={executeOperations[currentOperation]}
      flow={currentOperation}
      formatter={duskFormatter}
      {gasLimits}
      {gasSettings}
      minAllowedStake={luxToDusk(minimumStake)}
      on:operationChange
      on:suppressStakingNotice
      rewards={luxToDusk(stakeInfo.reward)}
      spendable={balance.unshielded.value}
      staked={stakeInfo.amount ? luxToDusk(stakeInfo.amount.total) : 0}
      {statuses}
      {hideStakingNotice}
    />
  {:else}
    <ContractStatusesList {statuses} />
    <ContractOperations items={operations} on:operationChange />
  {/if}
{/key}

<style>
  :global(.error-action) {
    width: 100%;
    text-align: left;
  }
</style>
