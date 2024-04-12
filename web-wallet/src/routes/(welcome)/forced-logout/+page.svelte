<script>
  import { onMount } from "svelte";
  import { mdiAlertOutline } from "@mdi/js";

  import { Card } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";

  onMount(() => {
    // eslint-disable-next-line no-alert
    alert(
      [
        "You have been automatically logged out as another ",
        "tab or window unlocked a different wallet.",
      ].join("")
    );
  });
</script>

<section class="forced-logout">
  <Card
    heading="You have been automatically logged out"
    iconPath={mdiAlertOutline}
  >
    <p>
      Another tab or window unlocked a different wallet and replaced the local
      cache.
    </p>
    <p>
      When you log in again with the mnemonic you used in this session, you will
      need to wait for a full sync.
    </p>
  </Card>
  <AppAnchorButton data-sveltekit-replacestate href="/" text="HOME" />
</section>

<style>
  .forced-logout {
    display: flex;
    flex-direction: column;
    flex: 1;
    justify-content: space-between;
  }

  p + p {
    margin-top: var(--default-gap);
  }
</style>
