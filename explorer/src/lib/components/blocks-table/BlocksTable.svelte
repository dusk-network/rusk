<svelte:options immutable={true} />

<script>
  import { AppAnchor } from "$lib/components";
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { Badge } from "$lib/dusk/components";
  import { luxToDusk } from "$lib/dusk/currency";
  import { getRelativeTimeString } from "$lib/dusk/string";
  import { createValueFormatter } from "$lib/dusk/value";

  import "./BlocksTable.css";

  /** @type {Block[]}*/
  export let data;

  const numberFormatter = createValueFormatter("en");
</script>

<Table>
  <TableHead>
    <TableRow>
      <TableCell type="th"># Block</TableCell>
      <TableCell type="th">Fee</TableCell>
      <TableCell type="th">Txn(s)</TableCell>
      <TableCell type="th">Rewards</TableCell>
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
          <small class="block__time"
            >{getRelativeTimeString(block.header.date, "long")}</small
          >
        </TableCell>
        <TableCell>
          <b class="block__fee-avg-label">AVG:</b>
          {numberFormatter(block.transactions.stats.averageGasPrice)}<br />
          <b class="block__fee-total-label">TOTAL:</b>
          {numberFormatter(block.transactions.stats.gasUsed)}
        </TableCell>
        <TableCell>{numberFormatter(block.transactions.data.length)}</TableCell>
        <TableCell
          ><Badge
            variant="alt"
            text={`${numberFormatter(luxToDusk(block.header.reward))} Dusk`}
          /></TableCell
        >
      </TableRow>
    {/each}
  </TableBody>
</Table>
