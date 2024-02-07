<svelte:options immutable={true}/>

<script>
	import { createEventDispatcher, onMount, tick } from "svelte";
	import { fade } from "svelte/transition";
	import { mdiDatabaseArrowDownOutline, mdiDatabaseOutline } from "@mdi/js";

	import { deductLuxFeeFrom } from "$lib/contracts";
	import { duskToLux, luxToDusk } from "$lib/dusk/currency";
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
		OperationResult
	} from "$lib/components";

	import StakeOverview from "./StakeOverview.svelte";

	/** @type {(...args: any[]) => Promise<string>} */
	export let execute;

	/** @type {StakeType} */
	export let flow;

	/** @type {(amount: number) => string} */
	export let formatter;

	/** @type {ContractGasSettings} */
	export let gasSettings;

	/** @type {number} */
	export let rewards;

	/** @type {number} */
	export let spendable;

	/** @type {number} */
	export let staked;

	/** @type {ContractStatus[]} */
	export let statuses;

	const defaultMinStake = 1000;

	/** @type {number} */
	let stakeAmount = {
		"stake": defaultMinStake,
		"withdraw-rewards": rewards,
		"withdraw-stake": staked
	}[flow];

	/** @type {HTMLInputElement|null} */
	let stakeInput;

	/** @type {boolean} */
	let isNextButtonDisabled = false;

	/** @type {boolean} */
	let validGasLimits = false;

	/** @type {number} */
	let gasPrice;

	/** @type {number} */
	let gasLimit;

	/** @type {Record<StakeType, string>} */
	const overviewLabels = {
		"stake": "Amount",
		"withdraw-rewards": "Withdraw Rewards",
		"withdraw-stake": "Withdraw Amount"
	};

	const checkAmountValid = async () => {
		await tick();
		isNextButtonDisabled = !(stakeInput?.checkValidity() && validGasLimits
			&& (luxFee + luxToDusk(stakeAmount) <= duskToLux(spendable)));
	};

	const dispatch = createEventDispatcher();
	const resetOperation = () => dispatch("operationChange", "");

	onMount(() => {
		if (flow === "stake") {
			stakeInput = document.querySelector(".operation__input-field");
			stakeAmount = Math.min(minStake, stakeAmount);
			checkAmountValid();
		}
	});

	$: luxFee = gasLimit * gasPrice;
	$: fee = formatter(luxToDusk(luxFee));
	$: maxSpendable = deductLuxFeeFrom(spendable, luxFee);
	$: minStake = maxSpendable > 0 ? Math.min(defaultMinStake, maxSpendable) : defaultMinStake;
</script>

<div class="operation">
	<Wizard steps={flow === "stake" ? 3 : 2} let:key>
		{#if flow === "stake"}
			<WizardStep
				step={0}
				{key}
				backButton={{
					action: resetOperation,
					disabled: false
				}}
				nextButton={{ disabled: isNextButtonDisabled }}>
				<ContractStatusesList items={statuses}/>
				<div class="operation__amount operation__space-between">
					<p>Enter amount:</p>
					<Button
						size="small"
						variant="tertiary"
						on:click={() => {
							if (stakeInput) {
								stakeInput.value = maxSpendable.toString();
							}

							stakeAmount = maxSpendable;
							checkAmountValid();
						}}
						text="USE MAX"
					/>
				</div>

				<div class="operation__amount operation__input">
					<Textbox
						className="operation__input-field"
						bind:value={stakeAmount}
						type="number"
						min={minStake}
						max={maxSpendable}
						required
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
					on:setGasSettings={(event) => {
						gasPrice = event.detail.price;
						gasLimit = event.detail.limit;
					}}
					on:checkGasLimits={(event) => {
						validGasLimits = event.detail;
						checkAmountValid();
					}}
				/>
			</WizardStep>
		{/if}

		<WizardStep
			step={flow === "stake" ? 1 : 0}
			{key}
			backButton={
				flow !== "stake"
					? { action: resetOperation, disabled: false }
					: undefined
			}
			nextButton={{
				disabled: stakeAmount === 0,
				icon: {
					path: flow === "stake" ? mdiDatabaseOutline : mdiDatabaseArrowDownOutline,
					position: "before"
				},
				label: flow === "stake" ? "STAKE" : "WITHDRAW",
				variant: "secondary"
			}}>
			<div in:fade|global class="operation__stake">
				<ContractStatusesList items={statuses}/>
				<Badge
					className="operation__rewards-notice"
					text="REVIEW TRANSACTION"
					variant="warning"
				/>
				<StakeOverview
					label={overviewLabels[flow]}
					value={formatter(stakeAmount)}
				/>

				{#if flow === "stake"}
					<GasFee {fee}/>
				{:else}
					<GasSettings
						{fee}
						limit={gasSettings.gasLimit}
						limitLower={gasSettings.gasLimitLower}
						limitUpper={gasSettings.gasLimitUpper}
						price={gasSettings.gasPrice}
						priceLower={gasSettings.gasPriceLower}
						on:setGasSettings={(event) => {
							gasPrice = event.detail.price;
							gasLimit = event.detail.limit;
						}}
						on:checkGasLimits={(event) => {
							validGasLimits = event.detail;
							checkAmountValid();
						}}
					/>
				{/if}

			</div>
		</WizardStep>

		<WizardStep
			step={flow === "stake" ? 2 : 1}
			{key}
			showNavigation={false}>

			<OperationResult
				errorMessage="Transaction failed"
				onBeforeLeave={resetOperation}
				operation={flow === "stake" ? execute(stakeAmount, gasLimit, gasPrice) : execute(gasLimit, gasPrice)}
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
		&__amount {
			display: flex;
			align-items: center;
			width: 100%;
		}

		&__stake {
			display: flex;
			flex-direction: column;
			gap: 1.2em;
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
	}
</style>
