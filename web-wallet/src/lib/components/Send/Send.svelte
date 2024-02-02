<svelte:options immutable={true}/>

<script>
	import { fade } from "svelte/transition";
	import { createEventDispatcher, onMount } from "svelte";
	import { mdiArrowUpBoldBoxOutline, mdiWalletOutline } from "@mdi/js";

	import { deductLuxFeeFrom } from "$lib/contracts";
	import { luxToDusk } from "$lib/dusk/currency";
	import { logo } from "$lib/dusk/icons";
	import {
		AnchorButton,
		Badge,
		Button,
		Icon,
		Textbox,
		Wizard,
		WizardStep
	} from "$lib/dusk/components";
	import {
		ContractStatusesList,
		GasFee,
		GasSettings,
		OperationResult,
		ScanQR
	} from "$lib/components";

	/** @type {(to: string, amount: number) => Promise<string>} */
	export let execute;

	/** @type {(amount: number) => string} */
	export let formatter;

	/** @type {ContractGasSettings} */
	export let gasSettings;

	/** @type {number} */
	export let spendable;

	/** @type {ContractStatus[]} */
	export let statuses;

	/** @type {number} */
	let amount = 1;

	/** @type {string} */
	let address = "";

	/** @type {import("qr-scanner").default} */
	let scanner;

	/** @type {import("..").ScanQR} */
	let scanQrComponent;

	/** @type {HTMLInputElement | null} */
	let amountInput;

	let isNextButtonDisabled = false;

	const checkAmountValid = () => {
		isNextButtonDisabled = !amountInput?.checkValidity();
	};

	const dispatch = createEventDispatcher();
	const resetOperation = () => dispatch("operationChange", "");

	onMount(() => {
		amountInput = document.querySelector(".operation__input-field");
		checkAmountValid();
	});

	$: luxFee = gasSettings.gasLimit * gasSettings.gasPrice;
	$: fee = formatter(luxToDusk(luxFee));
	$: maxSpendable = deductLuxFeeFrom(spendable, luxFee);
</script>

<div class="operation">
	<Wizard steps={4} let:key>
		<WizardStep
			step={0}
			{key}
			backButton={{
				action: resetOperation,
				disabled: false
			}}
			nextButton={{ disabled: isNextButtonDisabled }}
		>
			<div in:fade|global class="operation__send">
				<ContractStatusesList items={statuses}/>
				<div class="operation__send-amount operation__space-between">
					<p>Enter amount:</p>
					<Button
						size="small"
						variant="tertiary"
						on:click={() => {
							if (amountInput) {
								amountInput.value = maxSpendable.toString();
							}

							amount = maxSpendable;
							checkAmountValid();
						}}
						text="USE MAX"
					/>
				</div>

				<div class="operation__send-amount operation__input">
					<Textbox
						className="operation__input-field"
						bind:value={amount}
						required
						type="number"
						min={0.000000001}
						max={maxSpendable}
						step="0.000000001"
						on:input={checkAmountValid}
					/>
					<Icon
						data-tooltip-id="main-tooltip"
						data-tooltip-text="DUSK"
						path={logo}
					/>
				</div>

				<GasSettings
					{fee}
					limit={gasSettings.gasLimit}
					limitLower={gasSettings.gasLimitLower}
					limitUpper={gasSettings.gasLimitUpper}
					price={gasSettings.gasPrice}
					priceLower={gasSettings.gasPriceLower}
					on:setGasSettings
				/>
			</div>
		</WizardStep>
		<WizardStep
			step={1}
			{key}
			nextButton={{ disabled: address.length === 0 }}
		>
			<div in:fade|global class="operation__send">
				<ContractStatusesList items={statuses}/>

				<div class="operation__send-amount operation__space-between">
					<p>Enter address:</p>
					<Button
						disabled={!scanner}
						size="small"
						variant="secondary"
						on:click={() => {
							scanQrComponent.startScan();
						}}
						text="SCAN QR"
					/>
				</div>
				<Textbox
					className="operation__send-address"
					type="multiline"
					bind:value={address}
				/>
				<ScanQR
					bind:this={scanQrComponent}
					bind:scanner
					on:scan={(event) => {
						address = event.detail;
					}}
				/>
			</div>
		</WizardStep>
		<WizardStep
			step={2}
			{key}
			nextButton={{
				icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
				label: "SEND",
				variant: "secondary"
			}}
		>
			<div in:fade|global class="operation__send">
				<ContractStatusesList items={statuses}/>

				<Badge
					className="operation__review-notice"
					text="REVIEW TRANSACTION"
					variant="warning"
				/>

				<dl class="operation__review-transaction">
					<dt class="review-transaction__label">
						<Icon path={mdiArrowUpBoldBoxOutline}/>
						<span>Amount:</span>
					</dt>
					<dd class="review-transaction__value operation__review-amount">
						<span>{formatter(amount)}</span>
						<Icon
							className="dusk-amount__icon"
							path={logo}
							data-tooltip-id="main-tooltip"
							data-tooltip-text="DUSK"
						/>
					</dd>
				</dl>

				<dl class="operation__review-transaction">
					<dt class="review-transaction__label">
						<Icon path={mdiWalletOutline}/>
						<span>To:</span>
					</dt>
					<dd class="operation__review-address">
						<span>{address}</span>
					</dd>
				</dl>

				<GasFee {fee}/>
			</div>
		</WizardStep>
		<WizardStep step={3} {key} showNavigation={false}>
			<OperationResult
				errorMessage="Transaction failed"
				onBeforeLeave={resetOperation}
				operation={execute(address, amount)}
				pendingMessage="Processing transaction"
				successMessage="Transaction completed"
			>
				<svelte:fragment slot="success-content" let:result={hash}>
					{#if hash}
						<AnchorButton
							href={`https://explorer.dusk.network/transactions/transaction?id=${hash}`}
							on:click={resetOperation}
							text="VIEW ON BLOCK EXPLORER"
							variant="secondary"
							rel="noopener noreferrer"
							target="_blank"
						/>
					{/if}
				</svelte:fragment>
			</OperationResult>
		</WizardStep>
	</Wizard>
</div>

<style lang="postcss">
	.operation {
		&__review-address {
			background-color: transparent;
			border: 1px solid var(--primary-color);
			border-radius: 1.5em;
			padding: 0.75em 1em;
			width: 100%;
			line-break: anywhere;
		}

		&__review-transaction {
			display: flex;
			flex-direction: column;
			gap: 0.625em;
		}

		&__review-amount {
			justify-content: flex-start;
		}

		&__send {
			display: flex;
			flex-direction: column;
			gap: 1.2em;
		}

		&__send-amount {
			display: flex;
			align-items: center;
			width: 100%;
		}

		&__space-between {
			justify-content: space-between;
		}

		&__input {
			column-gap: var(--default-gap);
		}

		:global(&__input &__input-field) {
			width: 100%;
			padding: 0.5em 1em;
		}

		:global(&__input-field:invalid) {
			color: var(--error-color);
		}

		:global(&__send-address) {
			width: 100%;
		}

		:global(&__review-notice) {
			text-align: center;
		}

		:global(.dusk-amount__icon) {
			width: 1em;
			height: 1em;
			flex-shrink: 0;
		}
	}

	.review-transaction__label,
	.review-transaction__value {
		display: inline-flex;
		align-items: center;
		gap: var(--small-gap);
	}

	.review-transaction__value {
		font-weight: bold;
	}
</style>
