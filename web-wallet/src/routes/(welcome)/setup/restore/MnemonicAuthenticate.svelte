<script>
	import { Card, Mnemonic } from "$lib/dusk/components";

	/** @type {boolean} */
	export let isValid = false;

	/** @type {number} */
	export let wordLimit = 12;

	/** @type {string[]} */
	export let enteredMnemonicPhrase = [];

	$: isValid = enteredMnemonicPhrase.filter(word => word !== "").length === wordLimit;
</script>

<Card heading="Enter your Mnemonic Phrase">
	<div class="flex flex-col gap-1">
		<Mnemonic bind:enteredMnemonicPhrase {wordLimit} type="authenticate"/>
	</div>
</Card>
