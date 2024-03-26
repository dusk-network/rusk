<script>
  import { CopyField } from "$lib/components";
</script>

<section>
  <CopyField
    name="Sample Information"
    displayValue="Loading..."
    rawValue="Loading..."
    disabled={true}
  />

  <CopyField
    name="Sample Information"
    displayValue="1,2456,345"
    rawValue="12456345"
    disabled={false}
  />
</section>
