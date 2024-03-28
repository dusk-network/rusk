<script>
  import { Button, Card } from "$lib/dusk/components";
  import { AppAnchor, AppAnchorButton } from "$lib/components";

  /** @type {boolean} */
  export let tosAccepted = false;
</script>

<section class="flex flex-col flex-1">
  <Card heading="Terms &amp; Privacy Policy">
    <div class="flex flex-col gap-1">
      <p>
        Our <AppAnchor
          href="https://dusk.network/privacy-policy/"
          rel="noopener noreferrer"
          target="_blank">Terms &amp; Privacy Policy</AppAnchor
        > govern your use of our services, including data handling and user responsibilities.
        Your privacy and security are our top priorities.
      </p>
      <div class="flex gap-1 w-100">
        <AppAnchorButton
          className="flex-1"
          href="/setup"
          text="Cancel"
          variant="tertiary"
        />
        <Button
          className="flex-1"
          text="Accept"
          on:click={() => {
            tosAccepted = true;
          }}
        />
      </div>
    </div>
  </Card>
</section>
