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
  import { Badge } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";
  import {
    getRelativeTimeString,
    makeClassName,
    middleEllipsis,
  } from "$lib/dusk/string";
  import "./TransactionsTable.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction[]}*/
  export let data;

  const HASH_CHARS_LENGTH = 10;

  const numberFormatter = createValueFormatter("en");

  $: classes = makeClassName(["transactions-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">Hash</TableCell>
      <TableCell type="th">Gas</TableCell>
      <TableCell type="th">Fee</TableCell>
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
            >{middleEllipsis(
              transaction.blockhash,
              HASH_CHARS_LENGTH
            )}</AppAnchor
          >
          <small class="transaction__time"
            >{getRelativeTimeString(transaction.date, "long")}</small
          >
        </TableCell>
        <TableCell>
          <b class="transaction__fee-price-label">PRICE:</b>
          {numberFormatter(transaction.gasprice)}<br />
          <b class="transaction__fee-limit-label">LIMIT:</b>
          {numberFormatter(transaction.gaslimit)}
        </TableCell>
        <TableCell
          ><Badge
            variant="alt"
            text={`${numberFormatter(luxToDusk(transaction.feepaid))} Dusk`}
          /></TableCell
        >
        <TableCell>
          <Badge
            variant={transaction.success ? "success" : "error"}
            text={transaction.success ? "success" : "failed"}
          />
        </TableCell>
        <TableCell><Badge text={transaction.method} /></TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
