<script>
  /** @type {bigint} */
  export let eligibility;

  /** @type {(value: number | bigint) => string} */
  export let formatter;
</script>

<div class="stake-maturity-info">
  <p class="stake-maturity-info__heading">Stake maturity in progress...</p>
  <p class="stake-maturity-info__body">
    You will start participating in the consensus at block #{formatter(
      eligibility
    )}.
  </p>
</div>

<style lang="postcss">
  .stake-maturity-info {
    width: 100%;
    padding: 0 0.75em;
    padding-top: 0.625em;

    &__heading {
      font-weight: 500;
      line-height: 21px;
      letter-spacing: -1;
    }

    &__body {
      font-size: 0.875rem;
      line-height: 1.5;
    }
  }
</style>
