<script>
	import { Card, Words } from "$lib/dusk/components";
	import { mdiAlertOutline } from "@mdi/js";
	import { generateMnemonic } from "bip39";

	/** @type {string[]} */
	export let mnemonicPhrase;

	if (mnemonicPhrase.length === 0) {
		mnemonicPhrase = generateMnemonic().split(" ");
	}
</script>

<Card heading="Keep this SAFE" iconPath={mdiAlertOutline}>
	<div class="flex flex-col gap-1">
		<p>Please make sure to write your phrase down and save it in a secure location.</p>
		<Words words={mnemonicPhrase}/>
	</div>
</Card>
