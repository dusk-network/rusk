<svelte:options immutable={true}/>

<script>
	import { onDestroy } from "svelte";
	import { fade } from "svelte/transition";
	import {
		compose,
		filterWith,
		find,
		hasKeyValue,
		last,
		take
	} from "lamb";
	import {
		mdiDatabaseOutline,
		mdiSwapVertical
	} from "@mdi/js";

	import {
		AnchorButton,
		Card,
		Tabs,
		Throbber
	} from "$lib/dusk/components";
	import {
		StakeContract,
		TransferContract
	} from "$lib/containers";
	import {
		AddressPicker,
		Balance,
		Transactions
	} from "$lib/components";
	import {
		operationsStore,
		settingsStore,
		walletStore
	} from "$lib/stores";
	import { contractDescriptors } from "$lib/contracts";
	import { sortByHeightDesc } from "$lib/transactions";

	/** @type {import('./$types').PageData} */
	export let data;

	const { currentPrice } = data;
	const {
		currency,
		dashboardTransactionLimit,
		language
	} = $settingsStore;

	/** @type {(descriptors: ContractDescriptor[]) => ContractDescriptor[]} */
	const getEnabledContracts = filterWith(hasKeyValue("disabled", false));

	/** @type {(transactions: Transaction[]) => Transaction[]} */
	const getTransactionsShortlist = compose(
		take(dashboardTransactionLimit),
		sortByHeightDesc
	);

	/** @param {CustomEvent} event */
	function handleSetGasSettings ({ detail }) {
		settingsStore.update(store => ({
			...store,
			gasLimit: detail.limit,
			gasPrice: detail.price
		}));
	}

	/** @param {string} id */
	function updateOperation (id) {
		operationsStore.update((store) => ({
			...store,
			currentOperation: id
		}));
	}

	const enabledContracts = getEnabledContracts(contractDescriptors);
	const tabItems = enabledContracts.map(({ id, label }) => ({
		icon: { path: id === "transfer" ? mdiSwapVertical : mdiDatabaseOutline },
		id,
		label
	}));

	let selectedTab = tabItems[0]?.id ?? "";

	$: selectedContract = find(enabledContracts, hasKeyValue("id", selectedTab));
	$: ({ balance, currentAddress, addresses } = $walletStore);
	$: ({ currentOperation } = $operationsStore);

	onDestroy(() => {
		updateOperation("");
	});
</script>

<div class="dashboard-content">
	<h2 class="visible-hidden">Dashboard</h2>

	<AddressPicker
		{addresses}
		{currentAddress}
	/>

	<Balance
		fiatCurrency={currency}
		fiatPrice={currentPrice[currency.toLowerCase()]}
		locale={language}
		tokenCurrency="DUSK"
		tokens={balance.value}
	/>

	{#if selectedContract}
		<article class="tabs">
			<Tabs
				bind:selectedTab
				items={tabItems}
				on:change={() => updateOperation("")}
			/>
			<div
				class="tabs__panel"
				class:tabs__panel--first={selectedTab === enabledContracts[0].id}
				class:tabs__panel--last={selectedTab === last(enabledContracts).id}
			>
				{#key selectedTab}
					<div in:fade class="tabs__contract">
						<svelte:component
							descriptor={selectedContract}
							on:operationChange={({ detail }) => updateOperation(detail)}
							on:setGasSettings={handleSetGasSettings}
							this={selectedTab === "transfer" ? TransferContract : StakeContract}
						/>
					</div>
				{/key}
			</div>
		</article>
	{/if}

	{#if currentOperation === "" && selectedTab === "transfer" }
		{#await walletStore.getTransactionsHistory()}
			<Throbber className="loading"/>
		{:then transactions}
			<Transactions transactions={getTransactionsShortlist(transactions)}>
				<h3 class="h4" slot="heading">Transactions</h3>
				<AnchorButton
					className="view-transactions"
					slot="controls"
					href="/dashboard/transactions"
					text="View all transactions"
					variant="tertiary"
				/>
			</Transactions>
		{:catch e}
			<Card heading="Error getting transactions">
				<pre>{e}</pre>
			</Card>
		{/await}
	{/if}
</div>

<style lang="postcss">
	.dashboard-content {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 1.375rem;
		overflow-y: auto;
		flex: 1;
	}

	.tabs {
		&__panel {
			border-radius: var(--control-border-radius-size);
			background: var(--surface-color);
			transition: border-radius 0.4s ease-in-out;

			&--first {
				border-top-left-radius: 0;
			}

			&--last {
				border-top-right-radius: 0;
			}
		}

		&__contract {
			display: flex;
			flex-direction: column;
			padding: 1rem 1.375rem;
			gap: var(--default-gap);
		}
	}

	:global(.view-transactions) {
		width: 100%;
	}

	:global(.loading) {
		align-self: center;
	}
</style>
