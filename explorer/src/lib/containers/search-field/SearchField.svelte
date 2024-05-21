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
  function submitHandler() {
    if (/^([0-9a-fA-F]{64}|\d+)$/g.test(value)) {
      duskAPI
        .search($appStore.network, value)
        .then((data) => {
          const type = data.length ? data[0].type : undefined;
          switch (type) {
            case "block":
              resetField();
              goto(`/blocks/block?id=${data[0].id}`);
              break;
            case "transaction":
              resetField();
              goto(`/transactions/transaction?id=${data[0].id}`);
              break;
            default:
              dispatch("invalid", {
                query: value,
                res: data,
              });
              resetField();
          }
        })
        .catch((e) => {
          dispatch("invalid", { query: value, res: e });
          resetField();
        });
    } else {
      dispatch("invalid", {
        query: value,
        res: new Error("Invalid query value"),
      });
      resetField();
    }
  }
</script>

<form on:submit|preventDefault={submitHandler}>
  <TextboxAndButton
    bind:value
    placeholder="Block/Hash"
    icon={{
      path: mdiMagnify,
      position: "after",
      size: "normal",
    }}
  />
</form>
