<svelte:options immutable={true} />

<script>
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

  /** @typedef {"asc" | "desc"} SortDirection */

  const SortColumn = {
    OWNER: "owner",
    REWARD: "reward",
    SLASHES: "slashes",
    STAKE: "stake",
    STAKING_ADDRESS: "stakingAddress",
  };

  /** @type {{ column: string, direction: SortDirection }} */
  let sort = {
    column: SortColumn.STAKE,
    direction: "desc", // highest stake first
  };

  /** @param {HostProvisioner} p @returns {string} */
  const getOwnerAddress = (p) => Object.values(p.owner ?? {})[0] ?? "";

  /** @param {HostProvisioner} p @returns {boolean | string} */
  const hasSeparateOwner = (p) => {
    const owner = getOwnerAddress(p);
    return owner && owner !== p.key;
  };

  /** @param {SortDirection} direction @returns {SortDirection} */
  const flipDirection = (direction) => (direction === "asc" ? "desc" : "asc");

  /**
   * @param {string} column
   */
  function toggleSort(column) {
    sort = {
      column,
      direction:
        sort.column === column ? flipDirection(sort.direction) : "desc",
    };
  }

  /** @param {HostProvisioner} p @returns {number} */
  const slashes = (p) => (p.faults ?? 0) + (p.hard_faults ?? 0);

  /**
   * Map from column -> comparator
   * @type {Record<string, (a: HostProvisioner, b: HostProvisioner) => number>}
   */
  const columnComparators = {
    [SortColumn.STAKING_ADDRESS]: (a, b) => a.key.localeCompare(b.key),
    [SortColumn.OWNER]: (a, b) =>
      Number(hasSeparateOwner(a)) - Number(hasSeparateOwner(b)),
    [SortColumn.STAKE]: (a, b) => Number(a.amount) - Number(b.amount),
    [SortColumn.REWARD]: (a, b) => Number(a.reward) - Number(b.reward),
    [SortColumn.SLASHES]: (a, b) => slashes(a) - slashes(b),
  };

  const defaultComparator = () => 0;

  /**
   * @param {HostProvisioner} a
   * @param {HostProvisioner} b
   * @param {string} column
   * @param {SortDirection} direction
   * @returns {number}
   */
  function compare(a, b, column, direction) {
    const comparator = columnComparators[column] ?? defaultComparator;
    const result = comparator(a, b);

    return direction === "asc" ? result : -result;
  }

  $: classes = makeClassName(["provisioners-table", className]);

  $: sortArrow = sort.direction === "asc" ? "↑" : "↓";

  $: rankByKey = new Map(
    data
      .slice()
      .sort((a, b) => compare(a, b, SortColumn.STAKE, "desc"))
      .map((p, index) => [p.key, index + 1])
  );

  $: sortedData = data
    .slice()
    .sort((a, b) => compare(a, b, sort.column, sort.direction));
</script>

<Table className={classes}>
  <TableHead>
    <TableRow>
      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.STAKE)}
        >
          <span>#</span>
          {#if sort.column === SortColumn.STAKE}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>

      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.STAKING_ADDRESS)}
        >
          <span>Staking Address</span>
          {#if sort.column === SortColumn.STAKING_ADDRESS}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>

      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.OWNER)}
        >
          <span>Owner Key</span>
          {#if sort.column === SortColumn.OWNER}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>

      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.STAKE)}
        >
          <span>Stake</span>
          {#if sort.column === SortColumn.STAKE}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>

      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.REWARD)}
        >
          <span>Accumulated Reward</span>
          {#if sort.column === SortColumn.REWARD}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>

      <TableCell type="th">
        <button
          type="button"
          class="provisioners-table__header-button"
          on:click={() => toggleSort(SortColumn.SLASHES)}
        >
          <span>Slashes</span>
          {#if sort.column === SortColumn.SLASHES}
            <span class="provisioners-table__header-sort-indicator">
              {sortArrow}
            </span>
          {/if}
        </button>
      </TableCell>
    </TableRow>
  </TableHead>

  <TableBody>
    {#each sortedData as provisioner (provisioner.key)}
      {@const ownerAddress = getOwnerAddress(provisioner)}
      {@const consensusAddress = provisioner.key}
      {@const separate = ownerAddress && ownerAddress !== consensusAddress}
      {@const rank = rankByKey.get(provisioner.key)}

      <TableRow>
        <TableCell>{rank}</TableCell>

        <TableCell>
          <div class="provisioners-table__staking-address-wrapper">
            {middleEllipsis(consensusAddress, HASH_CHARS_LENGTH)}
            <CopyButton
              name="Provisioner's staking address"
              rawValue={consensusAddress}
              variant="secondary"
            />
          </div>
        </TableCell>

        <TableCell>
          <Badge
            data-tooltip-id="provisioners-tooltip"
            data-tooltip-text={separate
              ? `Owner: ${middleEllipsis(
                  ownerAddress,
                  HASH_CHARS_LENGTH
                )}\nConsensus: ${middleEllipsis(
                  consensusAddress,
                  HASH_CHARS_LENGTH
                )}`
              : `Consensus: ${middleEllipsis(
                  consensusAddress,
                  HASH_CHARS_LENGTH
                )}`}
            text={separate ? "Yes" : "No"}
          />
        </TableCell>

        <TableCell>
          <b class="provisioners-table__stake-data-label">Active:</b>
          {numberFormatter(luxToDusk(provisioner.amount))}
          <br />
          <b class="provisioners-table__stake-data-label">Inactive:</b>
          {numberFormatter(luxToDusk(provisioner.locked_amt))}
          <br />
          <b class="provisioners-table__stake-data-label">Maturity At:</b>
          #{numberFormatter(provisioner.eligibility)}
        </TableCell>

        {@const parts = fixedNumberFormatter(
          luxToDusk(provisioner.reward)
        ).split(".")}

        <TableCell>
          <span
            data-tooltip-id="main-tooltip"
            data-tooltip-place="top"
            data-tooltip-type="info"
            data-tooltip-text={`${numberFormatter(
              luxToDusk(provisioner.reward)
            )} DUSK`}
          >
            {parts[0]}.<span class="decimal-shadow">{parts[1]}</span>
          </span>
        </TableCell>

        <TableCell>
          <b class="provisioners-table__slash-data-label">Soft:</b>
          {numberFormatter(provisioner.faults)}
          <br />
          <b class="provisioners-table__slash-data-label">Hard:</b>
          {numberFormatter(provisioner.hard_faults)}
        </TableCell>
      </TableRow>
    {/each}
  </TableBody>
</Table>
