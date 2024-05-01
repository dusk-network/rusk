<script>
  import { Badge, Card } from "$lib/dusk/components";
  import { StatisticsPanel } from "$lib/containers";
  import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableRow,
  } from "$lib/components/table";
  import { apiBlocks, apiTransactions } from "$lib/mock-data";
  import { getRelativeTimeString, middleEllipsis } from "$lib/dusk/string";
  import { luxToDusk } from "$lib/dusk/currency";
  import { createValueFormatter } from "$lib/dusk/value";

  const numberFormatter = createValueFormatter("en");

  const ITEMS_TO_DISPLAY = 15;
  const HASH_CHARS_LENGTH = 10;
  const MS_IN_SEC = 1000;

  const blocksData = apiBlocks.data.blocks.slice(0, ITEMS_TO_DISPLAY);
  const transactionsData = apiTransactions.data.slice(0, ITEMS_TO_DISPLAY);

  /** @param {APIBlock} block */
  function formatBlock(block) {
    const reward = numberFormatter(luxToDusk(block.header.reward));
    const timeString = getRelativeTimeString(
      new Date(block.header.ts * MS_IN_SEC),
      "long"
    );
    return { ...block, reward, timeString };
  }

  /** @param {APITransaction} transaction */
  function formatTransaction(transaction) {
    const fee = numberFormatter(luxToDusk(transaction.feepaid));
    const timeString = getRelativeTimeString(
      new Date(transaction.blockts * MS_IN_SEC),
      "long"
    );
    return { ...transaction, fee, timeString };
  }

  const formattedBlocks = blocksData.map(formatBlock);
  const formattedTransactions = transactionsData.map(formatTransaction);
</script>

<section class="chain-info">
  <StatisticsPanel />
</section>

<section class="tables">
  <Card className="blocks">
    <div slot="header" class="blocks__header">
      <h1 class="blocks__header-title">Blocks</h1>
      <a href="/blocks">All Blocks</a>
    </div>
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
        {#each formattedBlocks as block (block)}
          <TableRow>
            <TableCell>
              {numberFormatter(block.header.height)}
              <small class="block__time">{block.timeString}</small>
            </TableCell>
            <TableCell>
              <b class="block__fee-avg-label">AVG:</b>
              {numberFormatter(block.transactions.stats.averageGasPrice)}<br />
              <b class="block__fee-total-label">TOTAL:</b>
              {numberFormatter(block.transactions.stats.gasUsed)}
            </TableCell>
            <TableCell
              >{numberFormatter(block.transactions.data.length)}</TableCell
            >
            <TableCell
              ><Badge variant="alt" text={`${block.reward} Dusk`} /></TableCell
            >
          </TableRow>
        {/each}
      </TableBody>
    </Table>
  </Card>

  <Card className="transactions">
    <div slot="header" class="transactions__header">
      <h1 class="transactions__header-title">Transactions</h1>
      <a href="/transactions">All Transactions</a>
    </div>
    <Table>
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
        {#each formattedTransactions as transaction (transaction)}
          <TableRow>
            <TableCell>
              {middleEllipsis(transaction.blockhash, HASH_CHARS_LENGTH)}
              <small class="transaction__time">{transaction.timeString}</small>
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
                text={`${transaction.fee} Dusk`}
              /></TableCell
            >
            <TableCell>
              <Badge
                variant={transaction.success ? "success" : "error"}
                text={transaction.success ? "success" : "failed"}
              />
            </TableCell>
            <TableCell><Badge text={transaction.contract} /></TableCell>
          </TableRow>
        {/each}
      </TableBody>
    </Table>
  </Card>
</section>

<style lang="postcss">
  .block__fee-avg-label,
  .block__fee-total-label,
  .transaction__fee-price-label,
  .transaction__fee-limit-label {
    font-weight: 500;
  }

  .chain-info {
    grid-template-columns: 1fr;
  }

  .blocks__header,
  .transactions__header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.125rem;
  }

  .blocks__header-title,
  .transactions__header-title {
    font-weight: 400;
    text-transform: uppercase;
    font-size: 1.125rem;
    line-height: 32.2px;
    letter-spacing: -1%;
  }

  .transaction__time,
  .block__time {
    display: block;
    font-size: 0.75rem;
    color: var(--color-text-secondary);
    margin-top: 0.2rem;
  }

  .tables {
    display: flex;
    gap: 1rem;
    margin-top: 2rem;
  }

  :global(.blocks),
  :global(.transactions) {
    width: 50%; /*  set the width to 50% */
    padding: 0;
  }

  @media (min-width: 768px) {
    .chain-info {
      display: flex;
      flex-wrap: wrap;
      gap: 1.875rem;
    }
  }
</style>
