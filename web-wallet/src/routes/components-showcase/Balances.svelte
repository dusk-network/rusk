<script>
  import { Balance } from "$lib/components";

  const balance = 20000000;
  const shieldedTokens = 10000000;
</script>

<section>
  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    tokens={balance}
  />

  <Balance
    fiatCurrency="GBP"
    fiatPrice={10}
    locale="en"
    tokenCurrency="DUSK"
    tokens={balance}
    shieldedTokensPercentage={(shieldedTokens / balance) * 100}
  />
</section>
