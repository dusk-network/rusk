<section class="block">
  <div class="block__details">Block details</div>
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
