<svelte:options immutable={true}/>

<script>
	import {
		allOf,
		collect,
		getKey,
		hasKeyValue,
		map,
		pick,
		setKey,
		when
	} from "lamb";

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
		Receive,
		Send
	} from "$lib/components";

	/** @type {ContractDescriptor} */
	export let descriptor;

	/** @type {(to: string, amount: number, gasPrice: number, gasLimit: number) => Promise<string>} */
	const executeSend = (to, amount, gasPrice, gasLimit) =>
		walletStore.transfer(to, amount, gasPrice, gasLimit).then(getLastTransactionHash);

	const collectSettings = collect([
		pick(["gasLimit", "gasLimitLower", "gasLimitUpper", "gasPrice", "gasPriceLower"]),
		getKey("language")
	]);
	const isEnabledSend = allOf([
		hasKeyValue("disabled", false),
		hasKeyValue("id", "send")
	]);

	$: ({ currentOperation } = $operationsStore);
	$: [
		gasSettings,
		language
	] = collectSettings($settingsStore);
	$: ({
		balance,
		currentAddress,
		error,
		isSyncing
	} = $walletStore);
	$: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
	$: statuses = [{
		label: "Spendable",
		value: duskFormatter(balance.maximum)
	}];

	/**
	 * We want to update the disabled status ourselves only
	 * when the send operation is enabled in the descriptor;
	 * otherwise the descriptor takes precedence.
	 */
	$: operations = map(
		descriptor.operations,
		when(isEnabledSend, setKey("disabled", isSyncing || !!error))
	);
</script>

{#if currentOperation === "send"}
	<Send
		execute={executeSend}
		formatter={duskFormatter}
		{gasSettings}
		on:operationChange
		spendable={balance.maximum}
		{statuses}
	/>
{:else if currentOperation === "receive"}
	<Receive on:operationChange address={currentAddress}/>
{:else}
	<ContractStatusesList items={statuses}/>
	<ContractOperations items={operations} on:operationChange/>
{/if}
