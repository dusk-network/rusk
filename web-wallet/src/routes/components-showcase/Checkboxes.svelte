<script>
  import { Agreement, Checkbox } from "$lib/dusk/components";
</script>

<section>
  <p>Checkbox – enabled – unchecked</p>
  <Checkbox name="checkbox_one" />
  <p>Checkbox – enabled – checked</p>
  <Checkbox name="checkbox_two" checked />
  <p>Checkbox – disabled – unchecked</p>
  <Checkbox name="checkbox_three" disabled />
  <p>Checkbox – disabled – checked</p>
  <Checkbox name="checkbox_four" disabled checked />
  <p>Agreement – enabled</p>
  <Agreement
    checked={false}
    name="checkbox_five"
    label="I understand that without my seed phrase I
			cannot access my wallet and its assets."
  />
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    align-items: start !important;
  }
</style>
