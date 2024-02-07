<script>
	import { getContext } from "svelte";
	import { Button, Stepper } from "$lib/dusk/components";
	import { mdiArrowLeft, mdiArrowRight } from "@mdi/js";

	import { AppAnchorButton } from "$lib/components";

	/** @type {Number} */
	export let step;
	export let key;

	/** @type {Boolean} */
	export let showStepper = false;
	export let showNavigation = true;

	export let hideBackButton = false;

	/** @type {WizardButtonProps | Undefined} */
	export let backButton = undefined;

	/** @type {WizardButtonProps | Undefined} */
	export let nextButton = undefined;

	const wizardStore = getContext(key);

	$: ({ stepsCount, currentStep } = $wizardStore);

	function handleBack () {
		backButton?.action?.();
		wizardStore.decrementStep();
	}

	function handleNext () {
		nextButton?.action?.();
		wizardStore.incrementStep();
	}

	/**
	 * Returns the icon for the wizard button
	 * It can be:
	 * – The default icon for the
	 * wizard button ("back" or "next"), based on the button,
	 * if no icon is provided
	 * – No icon at all, if the icon prop is set to "null"
	 * - A custom icon, provided by the user
	 * @param {WizardButtonProps | Undefined} buttonProps
	 * @param {String} defaultIconPath
	 * @param {Boolean} isNextButton
	 * @returns {IconProp | Undefined}
	 */
	function getButtonIcon (buttonProps, defaultIconPath, isNextButton) {
		if (buttonProps?.icon === null) {
			return undefined;
		}

		return buttonProps?.icon ?? {
			path: defaultIconPath,
			position: isNextButton ? "after" : "before",
			size: "normal"
		};
	}

	/**
	 * Returns the common props for the wizard buttons
	 * @param {WizardButtonProps | Undefined} wizardButtonProps
	 * @param {String} defaultLabel
	 * @param {String} defaultIconPath
	 */
	function getButtonProps (wizardButtonProps, defaultLabel, defaultIconPath, isNextButton = false) {
		const stepCondition = isNextButton ? currentStep + 1 === stepsCount : currentStep === 0;

		return {
			disabled: wizardButtonProps?.disabled ?? stepCondition,
			icon: getButtonIcon(wizardButtonProps, defaultIconPath, isNextButton),
			text: wizardButtonProps?.label ?? defaultLabel,
			variant: wizardButtonProps?.variant ?? "tertiary"
		};
	}
</script>

{#if step === currentStep}
	<slot name="heading"/>
	{#if showStepper}
		<Stepper steps={stepsCount} activeStep={currentStep}/>
	{/if}
	<slot></slot>

	{#if showNavigation}
		<slot name="navigation">
			<div class="dusk-wizard__step-navigation">
				{#if !hideBackButton}
					{#if backButton?.isAnchor}
						<AppAnchorButton
							{...getButtonProps(backButton, "Back", mdiArrowLeft)}
							href={backButton?.href ?? "#"}
						/>
					{:else}
						<Button
							{...getButtonProps(backButton, "Back", mdiArrowLeft)}
							on:click={handleBack}
						/>
					{/if}
				{/if}

				{#if nextButton?.isAnchor}
					<AppAnchorButton
						{...getButtonProps(nextButton, "Next", mdiArrowRight, true)}
						href={nextButton?.href ?? "#"}
					/>
				{:else}
					<Button
						{...getButtonProps(nextButton, "Next", mdiArrowRight, true)}
						on:click={handleNext}
					/>
				{/if}
			</div>
		</slot>
	{/if}
{/if}
