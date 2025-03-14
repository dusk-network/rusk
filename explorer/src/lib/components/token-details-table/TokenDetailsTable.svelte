<svelte:options immutable={true} />

<script>
  import { makeClassName, middleEllipsis } from "$lib/dusk/string";
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { createValueFormatter } from "$lib/dusk/value";
  import { luxToDusk } from "$lib/dusk/currency";
  import {
    AppAnchor,
    TransactionStatus,
    TransactionType,
  } from "$lib/components";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {Transaction[]} */
  export let data;

  /** @type {boolean} */
  export let displayTooltips = false;

  const HASH_CHARS_LENGTH = 10;

  const formatter = createValueFormatter("en");

  $: classes = makeClassName(["tokens-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">ID</TableCell>
      <TableCell type="th">From</TableCell>
      <TableCell type="th">To</TableCell>
      <TableCell type="th">Amount (Dusk)</TableCell>
      <TableCell type="th">Fee (Dusk)</TableCell>
      <TableCell type="th">Status</TableCell>
      <TableCell type="th">Type</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as transaction (transaction)}
      <TableRow>
        <TableCell
          >{middleEllipsis(transaction.txid, HASH_CHARS_LENGTH)}</TableCell
        >
        <TableCell>
          <AppAnchor href={`/account/?account=${transaction.from}`}>
            {middleEllipsis(
              transaction.from ? transaction.from : "",
              HASH_CHARS_LENGTH
            )}
          </AppAnchor>
        </TableCell>
        <TableCell>
          <AppAnchor href={`/account/?account=${transaction.to}`}>
            {middleEllipsis(
              transaction.to ? transaction.to : "",
              HASH_CHARS_LENGTH
            )}
          </AppAnchor>
        </TableCell>
        <TableCell>
          {transaction.amount
            ? formatter(luxToDusk(transaction.amount))
            : "N/A"}
        </TableCell>
        <TableCell>{formatter(luxToDusk(transaction.gasprice))}</TableCell>
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
