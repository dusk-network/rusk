<script>

	/**
	 * The number of steps – should be greater or equal to two.
	 * @type {number}
	 */
	export let steps;

	/** @type {number} */
	export let activeStep;

	/**
	 * Calculates the progress percentage based on the active step in a stepper.
	 *
	 * @constant
	 * @type {string}
	 * @example
	 * // If there are 5 steps in total and the active step is 2
	 * // progress will be "width: 50%;"
	 *
	 * @param {number} steps - Total number of steps in the stepper.
	 * @param {number} activeStep - The current active step index (starting from 0).
	 */
	$: progress = `width: ${(100 / (steps - 1)) * activeStep}%;`;
</script>

<div class="dusk-stepper" role="tablist">
	<div class="dusk-stepper__progress-bar">
		<div class="dusk-stepper__progress-filler" style={progress}/>
	</div>

	{#if steps >= 2}
		<div class="dusk-stepper__steps">
			{#each Array(steps).keys() as currentStep (currentStep)}
				<div
					class="dusk-stepper__step"
					class:dusk-stepper__step--processed={currentStep
						<= activeStep}
					aria-selected={currentStep === activeStep}
					aria-disabled="true"
				/>
			{/each}
		</div>
	{/if}
</div>
