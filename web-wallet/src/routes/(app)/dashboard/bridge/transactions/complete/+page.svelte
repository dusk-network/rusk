<svelte:options immutable={true} />

<script>
  import { mdiHistory } from "@mdi/js";

  import { page } from "$app/stores";

  import { AnchorButton, ErrorDetails, Icon } from "$lib/dusk/components";
  import { AppAnchor } from "$lib/components";
  import { MESSAGES } from "$lib/constants";

  // @ts-ignore
  const hash = $page.state?.hash ?? null;
</script>

<div class="transaction-complete">
  <article class="bridge">
    <header class="bridge__header">
      <h3 class="h4">Bridge</h3>
      <div class="bridge__header-icons">
        <AppAnchor href="/dashboard/bridge/transactions">
          <Icon path={mdiHistory} />
        </AppAnchor>
      </div>
    </header>
    <div>
      {#if hash}
        <p>{MESSAGES.TRANSACTION_CREATED}</p>
        <br />
        <AnchorButton
          href={`/explorer/transactions/transaction?id=${hash}`}
          text="VIEW ON BLOCK EXPLORER"
          rel="noopener noreferrer"
          target="_blank"
        />
      {:else}
        <ErrorDetails
          summary="It has not been possible to finalize your withdrawal. Check your connected web 3 wallet for more details"
          error={new Error("Withdraw finalization failed.")}
        />
      {/if}
    </div>
  </article>
</div>

<style lang="postcss">
  .transaction-complete {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.375rem;
    overflow-y: auto;
    flex: 1;
  }

  .bridge {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding: 1.25em;

    &__header {
      display: flex;
      justify-content: space-between;
    }

    &__header-icons {
      display: flex;
      align-items: center;
      gap: 0.675em;
    }
  }
</style>
