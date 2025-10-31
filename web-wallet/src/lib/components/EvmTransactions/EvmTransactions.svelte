<script>
  import { compose, getKey, take } from "lamb";
  import { mdiArrowLeft, mdiContain } from "@mdi/js";
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";

  import { AppAnchorButton } from "$lib/components";
  import {
    Anchor,
    Badge,
    Button,
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
  import { networkStore, walletStore } from "$lib/stores";
  import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

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

  const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;

  /** @type {Number} */
  let screenWidth = window.innerWidth;

  /** @type {bigint} */
  let currentBlockHeight;

  /** @type {(transactions: Transaction[]) => Transaction[]} */
  const getOrderedTransactions = limit
    ? compose(take(limit), sortByHeightDesc)
    : sortByHeightDesc;

  async function getFinalizationPeriod() {
    const contract = await walletStore.useContract(VITE_BRIDGE_CONTRACT_ID, wasmPath);
    return await contract.call.finalization_period();
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  $: ({ currentProfile } = $walletStore);
  $: {
    (async () => {
      currentBlockHeight = await networkStore.getCurrentBlockHeight();
      console.log(currentBlockHeight);
    })();
  }
</script>

<article in:fade|global class="transactions">
  <header class="transactions__header">
    <h3 class="h4">Pending Withdrawals</h3>
    <AppAnchorButton
      className="transactions__footer-button"
      href="/dashboard/bridge"
      text="Back"
      variant="tertiary"
      icon={{ path: mdiArrowLeft }}
    />
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
        {#each transactions as transaction}
          {#if transaction[1].to.External === currentProfile.account.toString()}
            <dl class="transactions-list">
              <dt class="transactions-list__term">Block</dt>
              <dd class="transactions-list__datum">
                {new Intl.NumberFormat(language).format(transaction[1].block_height)}
              </dd>
              <dt class="transactions-list__term">Amount</dt>
              <dd class="transactions-list__datum">
                {transferFormatter(transaction[1].amount)}
                <span class="transactions-list__ticker">Dusk</span>
              </dd>
              <dt class="transactions-list__term">From</dt>
              <dd class="transactions-list__datum">
                {middleEllipsis(
                  transaction[1].from,
                  calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
                )}
              </dd>
              {#await getFinalizationPeriod() then finalizationPeriod}
                {#if Number(currentBlockHeight) > Number(transaction[1].block_height) + Number(finalizationPeriod)}
                  <dt class="transactions-list__term">Status</dt>
                  <dd class="transactions-list__datum">
                    <Button
                      text="Finalize now"
                      on:click={() => {
                        const hash = walletStore.finalizeWithdrawalEvmFunctionCall(
                          VITE_BRIDGE_CONTRACT_ID,
                          transaction[0],
                          wasmPath
                        ).then(getKey("hash"));

                        console.log({ hash });
                      }}
                    />
                  </dd>
                {/if}
              {/await}
            </dl>
          {/if}
        {/each}
      {:else}
        <div class="transactions-list__empty">
          <Icon path={mdiContain} size="large" />
          <p>You have no pending withdrawals</p>
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
