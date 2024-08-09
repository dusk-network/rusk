<svelte:options immutable={true} />

<script>
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { AppAnchor } from "$lib/components";
  import { Badge, RelativeTime } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import { makeClassName, middleEllipsis } from "$lib/dusk/string";
  import "./TransactionsTable.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction[]}*/
  export let data;

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
          <AppAnchor
            className="transaction__link"
            href={`/transactions/transaction?id=${transaction.txid}`}
            >{middleEllipsis(transaction.txid, HASH_CHARS_LENGTH)}</AppAnchor
          >
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
          <Badge
            variant={transaction.success ? "success" : "error"}
            text={transaction.success ? "success" : "failed"}
          />
        </TableCell>
        <TableCell
          ><Badge
            className="transaction__type"
            text={transaction.method}
          /></TableCell
        >
      </TableRow>
    {/each}
  </TableBody>
</Table>
