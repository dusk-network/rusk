<script>
  import { Balance } from "$lib/components";

  const shieldedAmount = 1_000_000_000n;
  const unshieldedAmount = 2_000_000_000n;

  const shieldedAmountEx2 = 1_000_000_000n;
  const unshieldedAmountEx2 = 1_000_000_000n;
</script>

<section>
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    {shieldedAmount}
    {unshieldedAmount}
  />
  <br />
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    shieldedAmount={shieldedAmountEx2}
    unshieldedAmount={unshieldedAmountEx2}
  />
  <br />
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    shieldedAmount={shieldedAmountEx2}
    unshieldedAmount={0n}
  />
</section>
