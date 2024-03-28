<script>
  import { AnchorButton, Wizard, WizardStep } from "$lib/dusk/components";
  import { mdiSendCheck } from "@mdi/js";
  function progressStepOne() {
    // eslint-disable-next-line no-console
    console.log("Step 1");
  }
</script>

<section>
  <p>Wizard with stepper and headings</p>
  <Wizard steps={3} let:key>
    <WizardStep
      showStepper
      step={0}
      {key}
      backButton={{
        disabled: false,
        isAnchor: true,
      }}
      nextButton={{
        action: progressStepOne,
        icon: {
          path: mdiSendCheck,
          position: "after",
        },
        label: "Go Next",
        variant: "primary",
      }}
    >
      <h2 slot="heading">Step 1</h2>
      <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
    </WizardStep>
    <WizardStep showStepper step={1} {key}>
      <h2 slot="heading">Step 2</h2>
      <p>Mauris mollis felis enim, nec venenatis ante lobortis quis.</p>
    </WizardStep>
    <WizardStep showStepper step={2} {key}>
      <h2 slot="heading">All Done</h2>
      <p>Proin aliquet dolor pulvinar, blandit ante non, cursus leo.</p>
      <AnchorButton href="/" slot="navigation" text="Go Home" />
    </WizardStep>
  </Wizard>
</section>

<section>
  <p>Wizard without a stepper and headings</p>
  <Wizard steps={3} let:key>
    <WizardStep step={0} {key}>
      <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
    </WizardStep>
    <WizardStep step={1} {key}>
      <p>Mauris mollis felis enim, nec venenatis ante lobortis quis.</p>
    </WizardStep>
    <WizardStep step={2} {key}>
      <p>Proin aliquet dolor pulvinar, blandit ante non, cursus leo.</p>
      <AnchorButton href="/" slot="navigation" text="Go Home" />
    </WizardStep>
  </Wizard>
</section>

<style>
  section {
    width: 100%;
  }
</style>
