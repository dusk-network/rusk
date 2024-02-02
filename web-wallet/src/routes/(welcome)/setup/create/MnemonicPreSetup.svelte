<script>
	import {
		Agreement,
		Card,
		Icon
	} from "$lib/dusk/components";
	import { mdiAlertOutline } from "@mdi/js";

	/** @type {boolean} */
	export let isValid = false;

	/** @type {boolean} */
	let agreementOneChecked = isValid || false;

	/** @type {boolean} */
	let agreementTwoChecked = isValid || false;

	$: isValid = agreementOneChecked && agreementTwoChecked;
</script>

<Card heading="Securely store your seed phrase!">
	<div class="flex flex-col gap-1">
		<Agreement
			name="mnemonic_agreement_one"
			bind:checked={agreementOneChecked}
			label="I understand that without my seed phrase I cannot access my wallet and its assets."
		/>
		<Agreement
			name="mnemonic_agreement_two"
			bind:checked={agreementTwoChecked}
			label="I understand that securing my recovery phrase is my own responsibility.
				Only I have access to my recovery phrase"
		/>
	</div>
</Card>

<div class="notice">
	<Icon path={mdiAlertOutline} size="large"/>
	<p>To proceed, please check all the relevant boxes.
		Dusk will not save and cannot retrieve your passphrase.</p>
</div>
