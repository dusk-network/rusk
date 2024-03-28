<script>
  import { Button, Toast } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { mdiCopyleft } from "@mdi/js";
</script>

<section style="flex-direction:column">
  <Button
    text="toggle toast"
    size="small"
    on:click={() => toast("success", "Toast message", mdiCopyleft)}
  />

  <Button
    text="toggle another toast"
    size="small"
    on:click={() =>
      toast(
        "error",
        "This is a long message to test the wrapping of text",
        mdiCopyleft
      )}
  />

  <Toast />
</section>
