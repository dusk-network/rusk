<svelte:options immutable={true} />

<script>
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { luxToDusk } from "$lib/dusk/currency";
  import { makeClassName, middleEllipsis } from "$lib/dusk/string";
  import { createValueFormatter } from "$lib/dusk/value";

  import "./ProvisionersTable.css";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {HostProvisioner[]} */
  export let data;

  const HASH_CHARS_LENGTH = 10;

  const numberFormatter = createValueFormatter("en");

  $: classes = makeClassName(["provisioners-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">Staking Address</TableCell>
      <TableCell type="th">Stake Amount (DUSK)</TableCell>
      <TableCell type="th">Slashes</TableCell>
      <TableCell type="th">Accumulated Reward (DUSK)</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as provisioner (provisioner)}
      <TableRow>
        <TableCell>
          {middleEllipsis(provisioner.key, HASH_CHARS_LENGTH)}
        </TableCell>
        <TableCell>
          <b class="provisioners-table__staked-amount-type-label"
            >Reclaimable:</b
          >
          {numberFormatter(luxToDusk(provisioner.amount))}
          <br />
          <b class="provisioners-table__staked-amount-type-label">Locked:</b>
          {numberFormatter(luxToDusk(provisioner.locked_amt))}
        </TableCell>
        <TableCell>
          <b class="provisioners-table__slash-type-label">Soft:</b>
          {numberFormatter(provisioner.faults)}
          <br />
          <b class="provisioners-table__slash-type-label">Hard:</b>
          {numberFormatter(provisioner.hard_faults)}
        </TableCell>
        <TableCell>{numberFormatter(luxToDusk(provisioner.reward))}</TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
