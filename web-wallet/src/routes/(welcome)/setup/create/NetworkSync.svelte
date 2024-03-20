<script>
  import { AppAnchor } from "$lib/components";
  import { Card } from "$lib/dusk/components";
  import CopyField from "$lib/dusk/components/CopyField/CopyField.svelte";
  import { createNumberFormatter } from "$lib/dusk/number";
  import { settingsStore } from "$lib/stores";

  let currentBlock = 150640; // palceholder

  $: ({ language } = $settingsStore);

  let numberFormatter = createNumberFormatter(language);
</script>

<Card heading="Current Block Height">
  <div class="flex flex-col gap-1">
    <p>
      Store the current block height in case you want to resync from it next
      time you reset your wallet. This can significantly reduce the sync time.
    </p>

    <CopyField
      name="Block Height"
      displayValue={numberFormatter(currentBlock)}
      rawValue={String(currentBlock)}
    ></CopyField>

    <small>
      This can later be retrieved from Settings. Find out more in our <AppAnchor
        href="#"
        rel="noopener noreferrer"
        target="_blank">docs</AppAnchor
      >.
    </small>
  </div></Card
>
