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
      <TableCell type="th">Staked Amount (DUSK)</TableCell>
      <TableCell type="th">Reclaimable Staked Amount (DUSK)</TableCell>
      <TableCell type="th">Slashes</TableCell>
      <TableCell type="th">Hard Slashes</TableCell>
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
          {numberFormatter(luxToDusk(provisioner.amount))}
        </TableCell>
        <TableCell>
          {numberFormatter(luxToDusk(provisioner.locked_amt))}
        </TableCell>
        <TableCell>
          {numberFormatter(provisioner.faults)}
        </TableCell>
        <TableCell>{numberFormatter(provisioner.hard_faults)}</TableCell>
        <TableCell>{numberFormatter(luxToDusk(provisioner.reward))}</TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
