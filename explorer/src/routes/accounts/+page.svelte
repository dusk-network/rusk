<script>
  import { page } from "$app/stores";
  import { Card } from "$lib/dusk/components";
  import { AccountOverview, TransactionsCard } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getMoonlightAccountTransactions);
  const getTransactions = () => {
    if (key) {
      dataStore.reset();
      dataStore.getData(key);
    }
  };

  let errorFetchingAccountStatus = false;
  /** @type {AccountStatus|null} */
  let accountStatus = null;

  /** @param {string} accountKey */
  function fetchAccountStatus(accountKey) {
    accountStatus = null;
    duskAPI
      .getAccountStatus(accountKey)
      .then((status) => {
        accountStatus = status;
        errorFetchingAccountStatus = false;
      })
      .catch(() => {
        errorFetchingAccountStatus = true;
      });
  }

  $: key = $page.url.searchParams.get("key");
  $: if (key) {
    getTransactions();
    fetchAccountStatus(key);
  }
  $: ({ data, error, isLoading } = $dataStore);
  $: ({ isSmallScreen } = $appStore);
</script>

{#if key}
  <section>
    <article>
      <AccountOverview
        {errorFetchingAccountStatus}
        accountBalance={accountStatus?.balance}
        accountAddress={key}
      />
    </article>

    <TransactionsCard
      on:retry={getTransactions}
      txns={data}
      {error}
      loading={isLoading}
      {isSmallScreen}
    />
  </section>
{:else}
  <Card>
    <p>Account not provided.</p>
  </Card>
{/if}
