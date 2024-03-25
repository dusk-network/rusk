<section class="chain-info">
  <div class="chain-info__statistics">Statistics</div>
  <div class="chain-info__blocks">Blocks</div>
  <div class="chain-info__transactions">Transactions</div>
</section>

<style lang="postcss">
  .chain-info {
    display: flex;
    row-gap: 1.25em;
    column-gap: 1.875em;
    flex-wrap: wrap;

    &__statistics {
      flex: 0 0 100%;
    }

    &__blocks,
    &__transactions {
      flex: 1;
    }
  }

  @media (max-width: 768px) {
    .chain-info {
      &__statistics,
      &__blocks,
      &__transactions {
        flex: 0 0 100%;
      }
    }
  }
</style>
