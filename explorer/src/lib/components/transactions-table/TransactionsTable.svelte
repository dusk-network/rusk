<svelte:options immutable={true} />

<script>
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import {
    AppAnchor,
    TransactionStatus,
    TransactionType,
  } from "$lib/components";
  import { CopyButton, RelativeTime } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import { makeClassName, middleEllipsis } from "$lib/dusk/string";
  import "./TransactionsTable.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction[]}*/
  export let data;

  /** @type {boolean} */
  export let displayTooltips = false;

  /** @type {"compact" | "full"} */
  export let mode;

  const HASH_CHARS_LENGTH = 10;

  const numberFormatter = createValueFormatter("en");

  $: classes = makeClassName(["transactions-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">ID</TableCell>
      {#if mode === "full"}
        <TableCell type="th">Gas</TableCell>
      {/if}
      <TableCell type="th">Fee (DUSK)</TableCell>
      <TableCell type="th">Status</TableCell>
      <TableCell type="th">Type</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as transaction (transaction)}
      <TableRow>
        <TableCell>
          <div class="transaction__transaction-id-container">
            <AppAnchor
              className="transaction__link"
              href={`/transactions/transaction?id=${transaction.txid}`}
              >{middleEllipsis(transaction.txid, HASH_CHARS_LENGTH)}</AppAnchor
            >
            <CopyButton
              name="Transaction's ID"
              rawValue={transaction.txid}
              variant="secondary"
            />
          </div>
          <RelativeTime className="transaction__time" date={transaction.date} />
        </TableCell>
        {#if mode === "full"}
          <TableCell>
            <b class="transaction__fee-price-label">PRICE:</b>
            {numberFormatter(transaction.gasprice)}<br />
            <b class="transaction__fee-limit-label">LIMIT:</b>
            {numberFormatter(transaction.gaslimit)}
          </TableCell>
        {/if}
        <TableCell>{numberFormatter(luxToDusk(transaction.feepaid))}</TableCell>
        <TableCell>
          <TransactionStatus
            errorMessage={transaction.txerror}
            showErrorTooltip={false}
          />
        </TableCell>
        <TableCell>
          <TransactionType data={transaction} {displayTooltips} />
        </TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
