<svelte:options immutable={true}/>

<script>
	import { mdiArrowLeft } from "@mdi/js";
	import { Balance, Transactions } from "$lib/components";
	import { AnchorButton, Card, Throbber } from "$lib/dusk/components";
	import { settingsStore, walletStore } from "$lib/stores";
	import { sortByHeightDesc } from "$lib/transactions";

	/** @type {import('./$types').PageData} */
	export let data;

	const { currentPrice } = data;
	const { currency, language } = $settingsStore;

	$: ({ balance } = $walletStore);
</script>

<div class="transactions">
	<h2 class="visible-hidden">Transactions</h2>

	<Balance
		fiatCurrency={currency}
		fiatPrice={currentPrice[currency.toLowerCase()]}
		locale={language}
		tokenCurrency="DUSK"
		tokens={balance.value}
	/>

	{#await walletStore.getTransactionsHistory()}
		<Throbber className="loading"/>
	{:then transactions}
		{#if transactions.length}
			<Transactions transactions={sortByHeightDesc(transactions)}>
				<h3 class="h4" slot="heading">Transactions</h3>
			</Transactions>
		{:else}
			<Card heading="Transactions">
				<p>You have no transaction history</p>
			</Card>
		{/if}
	{:catch e}
		<Card heading="Error getting transactions">
			<pre>{e}</pre>
		</Card>
	{/await}
	<AnchorButton
		href="/dashboard"
		text="Back"
		variant="tertiary"
		icon={{ path: mdiArrowLeft }}
	/>
</div>

<style lang="postcss">
	.transactions {
		width: 100%;
		display: flex;
		flex-direction: column;
		gap: 1.375rem;
		overflow-y: auto;
		flex: 1;
	}

	:global(.loading) {
		align-self: center;
	}
</style>
