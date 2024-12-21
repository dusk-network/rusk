<svelte:options immutable={true} />

<script>
  import { get, writable } from "svelte/store";
  import {
    mdiCheckDecagramOutline,
    mdiFlaskEmpty,
    mdiFlaskEmptyOutline,
  } from "@mdi/js";

  import { Button, Stepper } from "$lib/dusk/components";

  const stepperA = writable({
    step: 1,
    steps: [
      { iconPath: mdiFlaskEmptyOutline, label: "foo bar baz" },
      { iconPath: mdiFlaskEmpty, label: "baz qux quux foo bar baz" },
    ],
  });
  const stepperB = writable({
    step: 2,
    steps: [
      { label: "foo" },
      { label: "bar" },
      { label: "baz" },
      { label: "qux" },
      { iconPath: mdiCheckDecagramOutline, label: "quux" },
    ],
  });
  const stepperC = writable({
    step: 3,
    steps: [
      { label: "foo" },
      { label: "bar" },
      { label: "baz" },
      { label: "qux" },
      { iconPath: mdiCheckDecagramOutline, label: "quux" },
    ],
  });
  const stepperD = writable({
    step: 2,
    steps: [
      { label: "foo" },
      { label: "bar" },
      { label: "baz" },
      { label: "qux" },
      { iconPath: mdiCheckDecagramOutline, label: "quux" },
    ],
  });

  /** @param {MouseEvent} evt */
  function handleStepChange(evt) {
    const btn = /** @type {HTMLButtonElement} */ (evt.currentTarget);
    const step = btn.matches("button:first-of-type") ? -1 : 1;
    const steppersContainer = btn.parentElement?.parentElement;
    const stepperStoreIdx = Array.prototype.indexOf.call(
      steppersContainer?.parentElement?.children,
      steppersContainer
    );
    const stepperStore = [stepperA, stepperB, stepperC, stepperD][
      stepperStoreIdx
    ];
    const { step: stepperStep, steps: stepperSteps } = get(stepperStore);
    const wantedStep = stepperStep + step;

    let newStep;

    if (wantedStep === stepperSteps.length) {
      newStep = 0;
    } else if (wantedStep === -1) {
      newStep = stepperSteps.length - 1;
    } else {
      newStep = wantedStep;
    }

    stepperStore.set({ step: newStep, steps: stepperSteps });
  }

  $: ({ step: stepA, steps: stepsA } = $stepperA);
  $: ({ step: stepB, steps: stepsB } = $stepperB);
  $: ({ step: stepC, steps: stepsC } = $stepperC);
  $: ({ step: stepD, steps: stepsD } = $stepperD);
</script>

<section class="steppers__section">
  <div class="steppers__container">
    <Stepper activeStep={stepA} steps={stepsA} />

    <div class="steppers__buttons">
      <Button on:click={handleStepChange} text="Previous" size="small" />
      <Button on:click={handleStepChange} text="Next" size="small" />
    </div>
  </div>

  <div class="steppers__container">
    <Stepper activeStep={stepB} steps={stepsB} showStepLabelWhenInactive />

    <div class="steppers__buttons">
      <Button on:click={handleStepChange} text="Previous" size="small" />
      <Button on:click={handleStepChange} text="Next" size="small" />
    </div>
  </div>

  <div class="steppers__container">
    <Stepper activeStep={stepC} showStepNumbers={false} steps={stepsC} />

    <div class="steppers__buttons">
      <Button on:click={handleStepChange} text="Previous" size="small" />
      <Button on:click={handleStepChange} text="Next" size="small" />
    </div>
  </div>

  <div class="steppers__container">
    <Stepper
      activeStep={stepD}
      showStepNumbers={false}
      steps={stepsD}
      variant="secondary"
    />

    <div class="steppers__buttons">
      <Button on:click={handleStepChange} text="Previous" size="small" />
      <Button on:click={handleStepChange} text="Next" size="small" />
    </div>
  </div>

  <div class="steppers__container">
    <Stepper activeStep={6} showStepNumbers={false} steps={8} />
  </div>
</section>

<style lang="postcss">
  :global {
    .steppers__section {
      flex-direction: column;
      gap: var(--large-gap) !important;
    }

    .steppers__container,
    .steppers__buttons {
      display: flex;
      gap: var(--default-gap);
    }

    .steppers__container {
      width: 100%;
      padding: 2rem;
      background-color: var(--surface-color);
      border-radius: var(--control-border-radius-size);
      flex-direction: column;
    }

    .steppers__buttons {
      justify-content: space-around;
    }
  }
</style>
