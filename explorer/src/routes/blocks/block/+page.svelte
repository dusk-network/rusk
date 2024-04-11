<script>
  import { BlockDetails } from "$lib/components";

  const data = {
    header: {
      feespaid: 0,
      hash: "2f37fec165e3891e6f6beb329f0262cd0538edcb478a3db2154381ecf45150aa",
      height: 488911,
      nextblockhash:
        "b95069bdd0ff5b564f8fd6fafb01c6fa75ea69ef6b0325115c65da171538f214",
      prevblockhash:
        "64ea046bf4a86e692d79dd8dccffa2114e2dc2c82e2d6cd737da4aa818629b6d",
      reward: 16000000000,
      seed: "8e3b249b5d2915ab190630c2a2e5c5d42d9d2ed8dea5a3702797eae3651e809a4cc9809047fecc602e4aa865fdb2fe33",
      statehash:
        "fb4c93bcc64914203b4312cf09e02c230ed3bc013408b124d5679c6cf583efd8",
      timestamp: "2024-04-16 09:26:17 +0000 UTC",
      ts: 1,
      version: "0",
    },
    transactions: {
      data: [],
      stats: {
        averageGasPrice: 0,
        gasLimit: 5000000000,
        gasUsed: 0,
      },
    },
  };
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails {data} />
  </div>
  <div class="block__transactions">Block transactions</div>
</section>

<style lang="postcss">
  .block {
    display: flex;
    flex-direction: column;
    row-gap: 1.875em;
  }

  @media (max-width: 768px) {
    .block {
      row-gap: 1.25em;
    }
  }
</style>
