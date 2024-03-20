<script>
    import { AppAnchor } from "$lib/components";
  import { Card, Icon, Textbox } from "$lib/dusk/components";
  import { mdiAlertOutline, mdiCubeOutline } from "@mdi/js";

  /** @type {number} */
  export let blockHeight = 0;

  /** @type {boolean} */
  export let isValid = false;

  /** @type {boolean} */
  export let isToggled = false;

  //  We have to ensure that blockHeight < 0, or not bigger than the current block height
  $: isValid =
    !isToggled || blockHeight !== 0;

  $: if (isToggled) {
    blockHeight = 0;
  }
</script>

<Card hasToggle bind:isToggled iconPath={mdiCubeOutline} heading="Block Height">
  <div class="flex flex-col gap-1">
    <Textbox
      type="number"
      autocomplete="new-password"
      bind:value={blockHeight}
      placeholder="Block Height"
    />
  </div>
</Card>

<div class="notice">
  <Icon path={mdiAlertOutline} size="large" />
  <p>
    Syncing from a custom block height is optional. Doing so can significantly reduce sync times. However, setting a wrong block can lead to wrong balance or missing transactions. Find out more in our <AppAnchor
    href="#"
    rel="noopener noreferrer"
    target="_blank">documentation</AppAnchor
  >.
  </p>
</div>