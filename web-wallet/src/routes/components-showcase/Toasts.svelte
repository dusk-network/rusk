<script>
  import { Button, Toast } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { mdiAlertCircleOutline, mdiCopyleft } from "@mdi/js";

  function triggerToast() {
    toast("success", "Toast message", mdiCopyleft);
  }

  function triggerToastFailure() {
    toast(
      "error",
      "This is a long message to test the wrapping of text",
      mdiAlertCircleOutline
    );
  }
</script>

<section style="flex-direction:column">
  <Button
    text="toggle toast"
    variant="secondary"
    size="small"
    on:click={triggerToast}
  />

  <Button
    text="toggle another toast"
    variant="secondary"
    size="small"
    on:click={triggerToastFailure}
  />

  <Toast />
</section>
