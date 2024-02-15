<svelte:options immutable={true}/>

<script>
	import {
		collect,
		getKey,
		hasKeyValue,
		map,
		mapWith,
		pick,
		setKey,
		when
	} from "lamb";
	import { mdiCloseThick } from "@mdi/js";

	import { createCurrencyFormatter } from "$lib/dusk/currency";
	import { getLastTransactionHash } from "$lib/transactions";
	import {
		operationsStore,
		settingsStore,
		walletStore
	} from "$lib/stores";
	import {
		ContractOperations,
		ContractStatusesList,
		Stake
	} from "$lib/components";
	import {
		ErrorDetails,
		Icon,
		Throbber
	} from "$lib/dusk/components";

	/** @type {ContractDescriptor} */
	export let descriptor;

	const collectSettings = collect([
		pick(["gasLimit", "gasLimitLower", "gasLimitUpper", "gasPrice", "gasPriceLower"]),
		getKey("language")
	]);

	/** @type {Record<string, (info: WalletStakeInfo) => boolean>} */
	const disablingConditions = {
		"stake": info => info.has_staked,
		"unstake": info => !info.has_staked,
		"withdraw-rewards": info => info.reward <= 0
	};

	/** @type {Record<StakeType, (...args: any[]) => Promise<string>>} */
	const executeOperations = {
		"stake": (amount, gasPrice, gasLimit) =>
			walletStore.stake(amount, gasPrice, gasLimit).then(getLastTransactionHash),
		"unstake": (gasPrice, gasLimit) =>
			walletStore.unstake(gasPrice, gasLimit).then(getLastTransactionHash),
		"withdraw-rewards": (gasPrice, gasLimit) =>
			walletStore.withdrawReward(gasPrice, gasLimit).then(getLastTransactionHash)
	};

	/** @type {(operations: ContractOperation[]) => ContractOperation[]} */
	const disableAllOperations = mapWith(setKey("disabled", true));

	/** @type {(operationId: string) => operationId is StakeType} */
	const isStakeOperation = operationId => [
		"stake",
		"unstake",
		"withdraw-rewards"
	].includes(operationId);

	/**
	 * We want to update the disabled status ourselves only
	 * when the operation is enabled in the descriptor;
	 * otherwise the descriptor takes precedence.
	 *
	 * @param {ContractOperation[]} operations
	 * @param {WalletStakeInfo} stakeInfo
	 * @returns {ContractOperation[]}
	 */
	const getOperations = (operations, stakeInfo) => map(
		operations,
		when(hasKeyValue("disabled", false), updateOperationDisabledStatus(stakeInfo))
	);

	/**
	 * @param {WalletStakeInfo} stakeInfo
	 * @param {number} spendable
	 * @returns {ContractStatus[]}
	 */
	const getStatuses = (stakeInfo, spendable) => [{
		label: "Spendable",
		value: duskFormatter(spendable)
	}, {
		label: "Total Locked",
		value: duskFormatter(stakeInfo.amount)
	}, {
		label: "Rewards",
		value: duskFormatter(stakeInfo.reward)
	}];

	/**
	 * @param {WalletStakeInfo} stakeInfo
	 * @returns {(operation: ContractOperation) => ContractOperation}
	 */
	const updateOperationDisabledStatus = stakeInfo => operation => ({
		...operation,
		disabled: disablingConditions[operation.id]?.(stakeInfo) ?? true
	});

	$: ({ currentOperation } = $operationsStore);
	$: [
		gasSettings,
		language
	] = collectSettings($settingsStore);
	const { hideStakingNotice } = $settingsStore;
	$: ({ balance, error, isSyncing } = $walletStore);
	$: isSyncOK = !(isSyncing || !!error);
	$: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

{#key currentOperation}
	{#await walletStore.getStakeInfo()}
		<Throbber className="stake-throbber"/>
	{:then stakeInfo}
		{@const statuses = getStatuses(stakeInfo, balance.maximum)}
		{@const operations = isSyncOK
			? getOperations(descriptor.operations, stakeInfo)
			: disableAllOperations(descriptor.operations)
		}
		{#if isStakeOperation(currentOperation)}
			<Stake
				execute={executeOperations[currentOperation]}
				flow={currentOperation}
				formatter={duskFormatter}
				{gasSettings}
				on:operationChange
				on:suppressStakingNotice
				rewards={stakeInfo.reward}
				spendable={balance.maximum}
				staked={stakeInfo.amount}
				{statuses}
				{hideStakingNotice}
			/>
		{:else}
			<ContractStatusesList items={statuses}/>
			<ContractOperations items={operations} on:operationChange/>
		{/if}
	{:catch stakeInfoError}
		<div class="fetch-stake-info-error">
			<Icon
				path={mdiCloseThick}
				size="large"
			/>
			<ErrorDetails
				error={stakeInfoError}
				summary="Failed to retrieve stake info"
			/>
		</div>
	{/await}
{/key}

<style lang="postcss">
	:global(.stake-throbber) {
		align-self: center;
	}

	.fetch-stake-info-error {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--default-gap);
	}
</style>
