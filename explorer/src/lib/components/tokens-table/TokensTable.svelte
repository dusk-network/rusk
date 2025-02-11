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
  import { makeClassName, middleEllipsis } from "$lib/dusk/string";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {Token[]} */
  export let data;

  const HASH_CHARS_LENGTH = 10;

  $: classes = makeClassName(["tokens-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">Token</TableCell>
      <TableCell type="th">Total Current Supply</TableCell>
      <TableCell type="th">Max Circulating Supply</TableCell>
      <TableCell type="th">Ticker</TableCell>
      <TableCell type="th">Contract ID</TableCell>
      <TableCell type="th">Price ($)</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as token (token)}
      <TableRow>
        <TableCell>
          <AppAnchor href={`/tokens/token?name=${token.token}`}
            >{token.token}</AppAnchor
          ></TableCell
        >
        <TableCell>{token.totalCurrentSupply}</TableCell>
        <TableCell>{token.maxCirculatingSupply}</TableCell>
        <TableCell>{token.ticker}</TableCell>
        <TableCell
          >{middleEllipsis(token.contractId, HASH_CHARS_LENGTH)}</TableCell
        >
        <TableCell>{token.price}</TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
