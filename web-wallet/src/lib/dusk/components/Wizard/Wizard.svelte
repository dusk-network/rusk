<script>
  import { setContext } from "svelte";
  import createWizardStore from "./createWizardStore";

  export let steps = 0;

  export let fullHeight = false;

  const storeKey = `DUSK:WIZARD:${Math.random().toString(36)}`;

  const wizardStore = createWizardStore(steps);
  setContext(storeKey, wizardStore);
</script>

<div class="dusk-wizard" class:dusk-wizard--full-height={fullHeight}>
  <slot key={storeKey} />
</div>
