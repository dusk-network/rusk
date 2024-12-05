<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { mdiDatabaseOutline } from "@mdi/js";
  import { StakeContract } from "$lib/containers";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { contractDescriptors, updateOperation } from "$lib/contracts";
  import { networkStore, settingsStore, walletStore } from "$lib/stores";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";
  import { collect, getKey, pick } from "lamb";
  import { ContractStatusesList, StakeMaturityInfo } from "$lib/components";
  import { createNumberFormatter } from "$lib/dusk/number";

  /** @type {bigint} */
  let currentBlockHeight;

  /**
   * @param {keyof SettingsStoreContent} property
   * @param {any} value
   */
  function updateSetting(property, value) {
    settingsStore.update((store) => ({
      ...store,
      [property]: value,
    }));
  }

  onDestroy(() => {
    updateOperation("");
  });

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

  networkStore.getCurrentBlockHeight().then((blockHeight) => {
    currentBlockHeight = blockHeight;
  });

  $: numberFormatter = createNumberFormatter(language);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);

  $: ({ balance, stakeInfo } = $walletStore);
  $: [gasSettings, language] = collectSettings($settingsStore);

  /** @returns {ContractStatus[]} */
  $: statuses = [
    {
      label: "Spendable",
      value: duskFormatter(luxToDusk(balance.unshielded.value)),
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

  $: shouldShowStakeEligibility = stakeInfo.amount && currentBlockHeight;
</script>

{#if import.meta.env.VITE_FEATURE_STAKE || false}
  <IconHeadingCard
    gap="large"
    heading="Stake"
    icons={[mdiDatabaseOutline]}
    reverse
  >
    <ContractStatusesList {statuses}>
      {#if shouldShowStakeEligibility}
        <StakeMaturityInfo
          isParticipatingInConsensus={stakeInfo?.amount?.eligibility !==
            undefined && currentBlockHeight > stakeInfo.amount.eligibility}
          eligibility={stakeInfo?.amount?.eligibility ?? 0n}
          formatter={numberFormatter}
        />
      {/if}
    </ContractStatusesList>
    <StakeContract
      {gasSettings}
      {stakeInfo}
      formatter={duskFormatter}
      descriptor={contractDescriptors[2]}
      on:operationChange={({ detail }) => updateOperation(detail)}
      on:suppressStakingNotice={() => updateSetting("hideStakingNotice", true)}
    />
  </IconHeadingCard>
{/if}
