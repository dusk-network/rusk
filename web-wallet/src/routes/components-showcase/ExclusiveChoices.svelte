<svelte:options immutable={true} />

<script>
  import { ExclusiveChoice } from "$lib/dusk/components";

  const options = [
    { disabled: false, label: "Apple", value: "apple" },
    { disabled: false, label: "Banana", value: "banana" },
    { disabled: true, label: "Lemon", value: "lemon" },
    {
      disabled: false,
      label: "A really big orange",
      value: "a really big orange",
    },
    { disabled: false, label: "Pear", value: "pear" },
  ];

  let selected = "banana";
</script>

<section>
  <ExclusiveChoice {options} bind:value={selected} />
</section>

<span>The selected value is {selected}</span>
