<script>
  import { compose, take } from "lamb";
  import { mdiArrowLeft, mdiContain } from "@mdi/js";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { logo } from "$lib/dusk/icons";
  import { Badge, Icon, Suspense } from "$lib/dusk/components";
  import {
    createFeeFormatter,
    createTransferFormatter,
  } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { sortByHeightDesc } from "$lib/transactions";

  import AppAnchor from "../AppAnchor/AppAnchor.svelte";
  import AppAnchorButton from "../AppAnchorButton/AppAnchorButton.svelte";

  /** @type {String} */
  export let language;

  /** @type {Number | Undefined} */
  export let limit = undefined;

  const transferFormatter = createTransferFormatter(language);
  const feeFormatter = createFeeFormatter(language);

  /** @type {Promise<Transaction[]>} */
  export let items;

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
        text="View all transactions"
        variant="tertiary"
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
    <svelte:fragment slot="success-content" let:result={transactions}>
      {#if transactions.length}
        {#each getOrderedTransactions(transactions) as transaction (transaction.id)}
          <dl class="transactions-list">
            <dt class="transactions-list__term">Hash</dt>
            <dd class="transactions-list__datum transactions-list__datum--hash">
              <samp>
                <AppAnchor
                  href="https://explorer.dusk.network/transactions/transaction?id={transaction.id}"
                  rel="noopener noreferrer"
                  target="_blank"
                >
                  {middleEllipsis(
                    transaction.id,
                    calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
                  )}
                </AppAnchor>
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
              <Icon
                className="transactions-list__icon"
                path={logo}
                data-tooltip-id="main-tooltip"
                data-tooltip-text="DUSK"
                data-tooltip-place="top"
              />
            </dd>
            {#if transaction.direction === "Out"}
              <dt class="transactions-list__term">Fee</dt>
              <dd class="transactions-list__datum">
                {feeFormatter(transaction.fee)}
                <Icon
                  className="transactions-list__icon"
                  path={logo}
                  data-tooltip-id="main-tooltip"
                  data-tooltip-text="DUSK"
                  data-tooltip-place="top"
                />
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
    padding: 1.375em 1em;

    &__header {
      & :global(h3) {
        line-height: 150%;
        margin-bottom: 0.625em;
      }
    }

    :global(.transactions__footer-button) {
      width: 100%;
    }
  }

  :global {
    .transactions-list__container {
      margin: 1em 0;
    }

    .transactions-list {
      display: grid;
      grid-template-columns: max-content auto;

      &__term {
        background-color: var(--background-color-alt);
        grid-column: 1;
        line-height: 130%;
        text-transform: capitalize;
        padding: 0.3125em 0.625em 0.3125em 1.375em;
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
        margin-bottom: 1em;
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
