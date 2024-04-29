<script>
  import { mdiMagnify } from "@mdi/js";
  import { TextboxAndButton } from "$lib/components";
  import { goto } from "$lib/navigation";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createEventDispatcher } from "svelte";

  /** @type {String}*/
  let value;

  const dispatch = createEventDispatcher();

  function resetField() {
    value = "";
  }

  /**
   * Function accepts 64 character long alphanumeric strings
   */
  async function submitHandler() {
    if (/^([0-9a-fA-F]{64}|\d+)$/g.test(value)) {
      await duskAPI
        .search($appStore.network, value)
        .then((data) => {
          const type = data.length !== 0 ? data[0].type : undefined;
          switch (type) {
            case "block":
              goto(`/blocks/block?id=${data[0].id}`);
              resetField();
              break;
            case "transaction":
              goto(`/transactions/transaction?id=${data[0].id}`);
              resetField();
              break;
            default:
              dispatch("invalid", { query: value, res: data });
              resetField();
          }
        })
        .catch((e) => {
          dispatch("invalid", { query: value, res: e });
          resetField();
        });
    }
  }
</script>

<form on:submit|preventDefault={submitHandler}>
  <TextboxAndButton
    bind:value
    placeholder="Txs/Hash"
    icon={{
      path: mdiMagnify,
      position: "after",
      size: "normal",
    }}
  />
</form>
