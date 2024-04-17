<script>
  import { Button, Card } from "$lib/dusk/components";
  import { AppAnchor, AppAnchorButton } from "$lib/components";

  /** @type {boolean} */
  export let tosAccepted = false;
</script>

<section>
  <Card heading="Terms &amp; Privacy Policy">
    <p>
      Our <AppAnchor
        href="https://dusk.network/privacy-policy/"
        rel="noopener noreferrer"
        target="_blank">Terms &amp; Privacy Policy</AppAnchor
      > govern your use of our services, including data handling and user responsibilities.
      Your privacy and security are our top priorities.
    </p>
    <div class="actions">
      <AppAnchorButton
        className="actions__action"
        href="/setup"
        text="Cancel"
        variant="tertiary"
      />
      <Button
        className="actions__action"
        text="Accept"
        on:click={() => {
          tosAccepted = true;
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
