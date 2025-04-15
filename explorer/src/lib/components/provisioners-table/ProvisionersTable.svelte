<svelte:options immutable={true} />

<script>
  import { ownPairs } from "lamb";

  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { Badge, CopyButton } from "$lib/dusk/components";
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

  const fixedNumberFormatter = createValueFormatter("en", 2, 2);

  $: classes = makeClassName(["provisioners-table", className]);
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">Staking Address</TableCell>
      <TableCell type="th">Owner</TableCell>
      <TableCell type="th">Stake</TableCell>
      <TableCell type="th">Slashes</TableCell>
      <TableCell type="th" align="right">Accumulated Reward</TableCell>
    </TableRow>
  </TableHead>
  <TableBody>
    {#each data as provisioner (provisioner)}
      {@const [ownerType, ownerValue] = ownPairs(provisioner.owner)[0]}
      <TableRow>
        <TableCell>
          <div class="provisioners-table__staking-address-wrapper">
            {middleEllipsis(provisioner.key, HASH_CHARS_LENGTH)}
            <CopyButton
              name="Provisioner's staking address"
              rawValue={provisioner.key}
              variant="secondary"
            />
          </div>
        </TableCell>
        <TableCell>
          <Badge
            data-tooltip-id="provisioners-tooltip"
            data-tooltip-text={ownerType === "Account"
              ? middleEllipsis(ownerValue, HASH_CHARS_LENGTH)
              : ownerValue}
            text={ownerType}
          />
        </TableCell>
        <TableCell>
          <b class="provisioners-table__stake-data-label">Active:</b>
          {numberFormatter(luxToDusk(provisioner.amount))}
          <br />
          <b class="provisioners-table__stake-data-label">Inactive:</b>
          {numberFormatter(luxToDusk(provisioner.locked_amt))}
          <br />
          <b class="provisioners-table__stake-data-label">Maturity At: </b>
          #{numberFormatter(provisioner.eligibility)}
        </TableCell>
        <TableCell>
          <b class="provisioners-table__slash-data-label">Soft:</b>
          {numberFormatter(provisioner.faults)}
          <br />
          <b class="provisioners-table__slash-data-label">Hard:</b>
          {numberFormatter(provisioner.hard_faults)}
        </TableCell>
        {@const parts = fixedNumberFormatter(
          luxToDusk(provisioner.reward)
        ).split(".")}
        <TableCell align="right">
          <span
            data-tooltip-id="main-tooltip"
            data-tooltip-place="top"
            data-tooltip-type="info"
            data-tooltip-text="{numberFormatter(
              luxToDusk(provisioner.reward)
            )} DUSK"
          >
            {parts[0]}.<span class="decimal-shadow">{parts[1]}</span>
          </span>
        </TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
