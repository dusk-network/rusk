<script>
  import { BigIntInput } from "$lib/components";

  let defaultValue = 0n;
  let customValue = 125235n;
</script>

<section>
  <BigIntInput bind:value={defaultValue} />
  <BigIntInput bind:value={customValue} />
</section>
