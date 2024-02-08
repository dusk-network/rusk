<svelte:options immutable={true}/>

<script>
	import { createEventDispatcher, onMount, tick } from "svelte";
	import { fade } from "svelte/transition";
	import { mdiAlertOutline, mdiDatabaseArrowDownOutline, mdiDatabaseOutline } from "@mdi/js";

	import { deductLuxFeeFrom } from "$lib/contracts";
	import { duskToLux, luxToDusk } from "$lib/dusk/currency";
	import { logo } from "$lib/dusk/icons";

	import {
		Agreement,
		Anchor,
		AnchorButton,
		Badge,
		Button,
		Card,
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

	/** @type {boolean} */
	export let hideStakingNotice;

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
	let isValidGas = true;

	/** @type {boolean} */
	let isNextButtonDisabled = false;

	let { gasLimit, gasPrice } = gasSettings;

	/** @type {Record<StakeType, string>} */
	const overviewLabels = {
		"stake": "Amount",
		"withdraw-rewards": "Withdraw Rewards",
		"withdraw-stake": "Withdraw Amount"
	};

	const checkAmountValid = async () => {
		await tick();
		isNextButtonDisabled = !(stakeInput?.checkValidity() && isValidGas
			&& (luxFee + duskToLux(stakeAmount) <= duskToLux(spendable)));
	};

	const dispatch = createEventDispatcher();
	const resetOperation = () => dispatch("operationChange", "");
	const suppressStakingNotice = () => dispatch("suppressStakingNotice");

	/**
	 * @param {{detail:{price:number, limit:number, isValidGas:boolean}}} event
	 */
	const setGasValues = (event) => {
		isValidGas = event.detail.isValidGas;

		if (event.detail.isValidGas) {
			gasPrice = event.detail.price;
			gasLimit = event.detail.limit;
		}

		checkAmountValid();
	};

	onMount(() => {
		if (flow === "stake") {
			stakeInput = document.querySelector(".operation__input-field");
			stakeAmount = Math.min(minStake, stakeAmount);
		}

		checkAmountValid();
	});

	$: luxFee = gasLimit * gasPrice;
	$: fee = formatter(luxToDusk(luxFee));
	$: maxSpendable = deductLuxFeeFrom(spendable, luxFee);
	$: minStake = maxSpendable > 0 ? Math.min(defaultMinStake, maxSpendable) : defaultMinStake;

	let hideStakingNoticeNextTime = false;

	function getWizardSteps () {
		if (flow === "stake") {
			return hideStakingNotice ? 3 : 4;
		}

		return 2;
	}
</script>

<div class="operation">
	<Wizard steps={getWizardSteps()} let:key>
		{#if flow === "stake"}
			{#if !hideStakingNotice}
				<!-- STAKING NOTICE STEP -->
				<WizardStep
					step={0}
					{key}
					backButton={{
						action: resetOperation,
						disabled: false
					}}
					nextButton={{
						action: () => {
							if (hideStakingNoticeNextTime) {
								suppressStakingNotice();
							}
						},
						icon: null, label: "Agree", variant: "secondary"
					}}>
					<Badge
						text="WARNING"
						variant="warning"
					/>
					<Card
						onSurface
						iconPath={mdiAlertOutline}
						heading="Only stake if you have a node set up!">
						<p class="staking-warning">
							I understand that I have set up a node properly, as described <Anchor
								class="staking-warning__step-node-setup-link"
								rel="noopener noreferrer"
								target="_blank"
								href="https://docs.dusk.network/getting-started/node-setup/overview">HERE</Anchor>, and that I will lose funds if I have not done so correctly.</p>

						<Agreement
							className="staking-warning__agreement"
							name="staking-warning"
							label="Don't show this step again."
							bind:checked={hideStakingNoticeNextTime}/>
					</Card>
				</WizardStep>
			{/if}

			<!-- ENTER STAKING AMOUNT STEP -->
			<WizardStep
				step={hideStakingNotice ? 0 : 1}
				{key}
				backButton={hideStakingNotice ? {
					action: resetOperation,
					disabled: false
				} : undefined}
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
					on:gasSettings={setGasValues}
				/>
			</WizardStep>
		{/if}

		<!-- OPERATION OVERVIEW STEP  -->
		<WizardStep
			step={flow === "stake" ? hideStakingNotice ? 1 : 2 : 0}
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
						on:gasSettings={setGasValues}
					/>
				{/if}

			</div>
		</WizardStep>

		<!-- OPERATION RESULT STEP  -->
		<WizardStep
			step={flow === "stake" ? hideStakingNotice ? 2 : 3 : 1}
			{key}
			showNavigation={false}>

			<OperationResult
				errorMessage="Transaction failed"
				onBeforeLeave={resetOperation}
				operation={flow === "stake" ? execute(stakeAmount, gasPrice, gasLimit)
					: execute(gasPrice, gasLimit)}
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

	.staking-warning {
		margin-bottom: 1em;
	}

	:global(.staking-warning__step-node-setup-link) {
		color: var(--secondary-color-variant-dark)
	}

	:global(.staking-warning__agreement) {
		align-items: baseline;
	}
</style>
