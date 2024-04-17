<script>
  import { Button, Card } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";
  import { mdiAlertOutline } from "@mdi/js";

  /** @type {boolean} */
  export let notice = false;
</script>

<section>
  <Card iconPath={mdiAlertOutline} heading="Existing Wallet Detected">
    <p>
      Initializing a new wallet will replace your existing local wallet cache,
      erasing any stored data. Ensure you have securely backed up your current
      wallet's seed phrase to prevent loss. Proceeding without a backup can lead
      to irreversible loss of access to your assets.
    </p>
    <div class="actions">
      <AppAnchorButton
        className="actions__action"
        href="/setup"
        text="Back"
        variant="tertiary"
      />
      <Button
        className="actions__action"
        text="Proceed"
        on:click={() => {
          notice = true;
        }}
      />
    </div>
  </Card>
</section>

<style lang="postcss">
  .actions {
    display: flex;
    justify-content: space-between;
    margin-top: var(--default-gap);
    gap: var(--default-gap);
  }

  :global(.actions__action) {
    flex: 1;
  }
</style>
