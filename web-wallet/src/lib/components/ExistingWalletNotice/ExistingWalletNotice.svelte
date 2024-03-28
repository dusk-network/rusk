<script>
  import { Button, Card } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";
  import { mdiAlertOutline } from "@mdi/js";

  /** @type {boolean} */
  export let notice = false;
</script>

<section class="flex flex-col flex-1">
  <Card iconPath={mdiAlertOutline} heading="Existing Wallet Detected">
    <div class="flex flex-col gap-1">
      <p>
        Initializing a new wallet will replace your existing local wallet cache,
        erasing any stored data. Ensure you have securely backed up your current
        wallet's seed phrase to prevent loss. Proceeding without a backup can
        lead to irreversible loss of access to your assets.
      </p>
      <div class="flex gap-1 w-100">
        <AppAnchorButton
          className="flex-1"
          href="/setup"
          text="Back"
          variant="tertiary"
        />
        <Button
          className="flex-1"
          text="Proceed"
          on:click={() => {
            notice = true;
          }}
        />
      </div>
    </div>
  </Card>
</section>
