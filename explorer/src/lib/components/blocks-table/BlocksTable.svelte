<svelte:options immutable={true} />

<script>
  import { RelativeTime } from "$lib/dusk/components";
  import { AppAnchor } from "$lib/components";
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { luxToDusk } from "$lib/dusk/currency";
  import { makeClassName } from "$lib/dusk/string";
  import { createValueFormatter } from "$lib/dusk/value";

  import "./BlocksTable.css";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {Block[]} */
  export let data;

  const numberFormatter = createValueFormatter("en");

  $: classes = makeClassName(["blocks-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">Block</TableCell>
      <TableCell type="th">Gas</TableCell>
      <TableCell type="th">Txn(s)</TableCell>
      <TableCell type="th">Rewards (Dusk)</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as block (block)}
      <TableRow>
        <TableCell>
          <AppAnchor
            className="block__link"
            href={`/blocks/block?id=${block.header.hash}`}
            >{numberFormatter(block.header.height)}</AppAnchor
          >
          <RelativeTime className="block__time" date={block.header.date} />
        </TableCell>
        <TableCell>
          <b class="block__fee-avg-label">AVG PRICE:</b>
          {numberFormatter(block.transactions.stats.averageGasPrice)}<br />
          <b class="block__fee-total-label">USED:</b>
          {numberFormatter(block.transactions.stats.gasUsed)}
        </TableCell>
        <TableCell>{numberFormatter(block.transactions.data.length)}</TableCell>
        <TableCell>{numberFormatter(luxToDusk(block.header.reward))}</TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
