<script>
  import { compose, take } from "lamb";
  import { mdiArrowLeft, mdiContain } from "@mdi/js";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import {
    Anchor,
    Badge,
    Icon,
    Suspense,
    Throbber,
  } from "$lib/dusk/components";
  import {
    createFeeFormatter,
    createTransferFormatter,
  } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { sortByHeightDesc } from "$lib/transactions";

  import AppAnchorButton from "../AppAnchorButton/AppAnchorButton.svelte";

  /** @type {String} */
  export let language;

  /** @type {Number | Undefined} */
  export let limit = undefined;

  const transferFormatter = createTransferFormatter(language);
  const feeFormatter = createFeeFormatter(language);

  /** @type {Promise<Transaction[]>} */
  export let items;

  /** @type {Boolean}*/
  export let isSyncing;

  /** @type {Error|null}*/
  export let syncError;

  /** @type {Number} */
  let screenWidth = window.innerWidth;

  /** @type {(transactions: Transaction[]) => Transaction[]} */
  const getOrderedTransactions = limit
    ? compose(take(limit), sortByHeightDesc)
    : sortByHeightDesc;

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });
</script>

<article in:fade|global class="transactions">
  <header class="transactions__header">
    <h3 class="h4">Transactions</h3>
    {#if limit}
      <AppAnchorButton
        className="transactions__footer-button"
        href="/dashboard/transactions"
        text="All transactions"
        variant="primary"
      />
    {:else}
      <AppAnchorButton
        className="transactions__footer-button"
        href="/dashboard"
        text="Back"
        variant="tertiary"
        icon={{ path: mdiArrowLeft }}
      />
    {/if}
  </header>
  <Suspense
    className="transactions-list__container"
    errorMessage="Error getting transactions"
    errorVariant="details"
    waitFor={items}
  >
    <svelte:fragment slot="pending-content">
      <div class="transactions-list__loading-container">
        {#if !isSyncing && !syncError}
          <Throbber />
        {:else}
          <p>Data will load after a successful sync.</p>
        {/if}
      </div>
    </svelte:fragment>
    <svelte:fragment slot="success-content" let:result={transactions}>
      {#if transactions.length}
        {#each getOrderedTransactions(transactions) as transaction (transaction.id)}
          <dl class="transactions-list">
            <dt class="transactions-list__term">Hash</dt>
            <dd class="transactions-list__datum transactions-list__datum--hash">
              <samp>
                <Anchor
                  href="/explorer/transactions/transaction?id={transaction.id}"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {middleEllipsis(
                    transaction.id,
                    calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
                  )}
                </Anchor>
              </samp>
            </dd>
            {#if transaction.tx_type}
              <dt class="transactions-list__term">Type</dt>
              <dd class="transactions-list__datum">
                <Badge
                  className="transactions-list__badge"
                  text={transaction.tx_type}
                />
              </dd>
            {/if}
            <dt class="transactions-list__term">Block</dt>
            <dd class="transactions-list__datum">
              {new Intl.NumberFormat(language).format(transaction.block_height)}
            </dd>
            <dt class="transactions-list__term">Amount</dt>
            <dd class="transactions-list__datum">
              {transferFormatter(transaction.amount)}
              <span class="transactions-list__ticker">Dusk</span>
            </dd>
            {#if transaction.direction === "Out"}
              <dt class="transactions-list__term">Fee</dt>
              <dd class="transactions-list__datum">
                {feeFormatter(transaction.fee)}
                <span class="transactions-list__ticker">Dusk</span>
              </dd>
            {/if}
          </dl>
        {/each}
      {:else}
        <div class="transactions-list__empty">
          <Icon path={mdiContain} size="large" />
          <p>You have no transaction history</p>
        </div>
      {/if}
    </svelte:fragment>
  </Suspense>
</article>

<style lang="postcss">
  .transactions {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding-top: 1.375em;

    &__header {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      padding: 0 1rem;
      gap: 0.625rem;
      flex-wrap: wrap;

      & :global(h3) {
        line-height: 150%;
      }
    }
  }

  :global {
    .transactions-list {
      display: grid;
      grid-template-columns: max-content auto;
      width: 100%;

      &__term {
        background-color: var(--background-color-alt);
        grid-column: 1;
        line-height: 130%;
        text-transform: capitalize;
        padding: 0.3125em 0.625em 0.3125em 1.375em;
      }

      &__ticker {
        text-transform: uppercase;
      }

      &__loading-container {
        margin: 1.25em 0;
      }

      &__datum {
        grid-column: 2;
        line-height: 150%;
        padding: 0.312em 0.625em;
        display: flex;
        align-items: center;
        gap: 0.625em;
        font-family: var(--mono-font-family);
        overflow: hidden;

        & samp {
          display: block;
          white-space: nowrap;
          overflow: hidden;
        }

        &--hash {
          justify-content: center;
        }
      }

      &__empty {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5em;
        margin: 1.25em 0;
      }

      &__badge {
        flex: 1;
      }

      & dt:first-of-type,
      & dd:first-of-type {
        padding-top: 1em;
      }

      & dt:last-of-type,
      & dd:last-of-type {
        padding-bottom: 1em;
      }

      & dt:first-of-type {
        border-top-right-radius: 2em;
      }

      & dt:last-of-type {
        border-bottom-right-radius: 2em;
      }
    }
  }
</style>
