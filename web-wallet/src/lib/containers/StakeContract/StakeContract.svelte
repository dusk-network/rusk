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
    Unstake,
  } from "$lib/components";
  import { mdiDatabaseArrowDownOutline, mdiGiftOpenOutline } from "@mdi/js";

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
    "claim-rewards": (info) => info.reward <= 0n,
    stake: (info) => !!info.amount,
    unstake: (info) => !info.amount || info.amount.total === 0n,
  };

  /** @type {Record<StakeType, (...args: any[]) => Promise<string>>} */
  const executeOperations = {
    "claim-rewards": (gasPrice, gasLimit) =>
      walletStore
        .claimRewards(
          $walletStore.stakeInfo.reward,
          new Gas({ limit: gasLimit, price: gasPrice })
        )
        .then(getKey("hash")),
    stake: (amount, gasPrice, gasLimit) =>
      walletStore
        .stake(amount, new Gas({ limit: gasLimit, price: gasPrice }))
        .then(getKey("hash")),
    unstake: (gasPrice, gasLimit) =>
      walletStore
        .unstake(new Gas({ limit: gasLimit, price: gasPrice }))
        .then(getKey("hash")),
  };

  /** @type {(operations: ContractOperation[]) => ContractOperation[]} */
  const disableAllOperations = mapWith(setKey("disabled", true));

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

  const getMaxUnstakeAmount = () => {
    if (currentOperation === "unstake") {
      return stakeInfo.amount ? stakeInfo.amount.total : 0n;
    }

    return stakeInfo.reward;
  };
  /**
   * @returns {(operation: ContractOperation) => ContractOperation}
   */
  const updateOperationDisabledStatus = () => (operation) => ({
    ...operation,
    disabled: disablingConditions[operation.id]?.(stakeInfo) ?? true,
  });

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

  $: ({ currentOperation } = $operationsStore);
  const { hideStakingNotice } = $settingsStore;
  $: ({ balance, minimumStake, syncStatus } = $walletStore);
  $: isSyncOK = !(syncStatus.isInProgress || !!syncStatus.error);
  $: if (!isSyncOK) {
    disableAllOperations(descriptor.operations);
  }
</script>

{#key currentOperation}
  {#if currentOperation === "stake"}
    <Stake
      execute={executeOperations[currentOperation]}
      formatter={duskFormatter}
      {gasLimits}
      {gasSettings}
      minStakeRequirement={minimumStake}
      on:operationChange
      on:suppressStakingNotice
      availableBalance={balance.unshielded.value}
      {statuses}
      {hideStakingNotice}
    />
  {:else if currentOperation === "unstake" || currentOperation === "claim-rewards"}
    <Unstake
      execute={executeOperations[currentOperation]}
      formatter={duskFormatter}
      {gasLimits}
      {gasSettings}
      on:operationChange
      {statuses}
      maxAmount={getMaxUnstakeAmount()}
      operationCtaLabel={currentOperation === "unstake"
        ? "Unstake"
        : "Claim Rewards"}
      operationCtaIconPath={currentOperation === "unstake"
        ? mdiDatabaseArrowDownOutline
        : mdiGiftOpenOutline}
      operationOverviewLabel={currentOperation === "unstake"
        ? "Unstake Amount"
        : "Rewards Amount"}
    />
  {:else}
    <ContractStatusesList {statuses} />
    <ContractOperations items={operations} on:operationChange />
  {/if}
{/key}
