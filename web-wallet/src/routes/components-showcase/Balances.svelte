<script>
  import { Balance } from "$lib/components";

  const shieldedBalance = 1_000_000_000n;
  const publicBalance = 2_000_000_000n;

  const shieldedBalanceEx2 = 1_000_000_000n;
  const publicBalanceEx2 = 1_000_000_000n;
</script>

<section>
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    {shieldedBalance}
    {publicBalance}
  />
  <br />
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    shieldedBalance={shieldedBalanceEx2}
    publicBalance={publicBalanceEx2}
  />
  <br />
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    shieldedBalance={shieldedBalanceEx2}
    publicBalance={0n}
  />
  <br />
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    shieldedBalance={0n}
    publicBalance={0n}
  />
</section>
