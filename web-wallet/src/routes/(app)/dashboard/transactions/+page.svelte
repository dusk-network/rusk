<svelte:options immutable={true}/>

<script>
	import { Balance, Transactions } from "$lib/components";
	import { settingsStore, walletStore } from "$lib/stores";

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

	<Transactions items={walletStore.getTransactionsHistory()} {language}/>
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
</style>
