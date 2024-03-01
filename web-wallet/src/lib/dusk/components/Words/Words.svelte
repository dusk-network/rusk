<script>
  import { makeClassName } from "$lib/dusk/string";

  /**
   * @type {string[]}
   */
  export let words;

  /** @type {String | Undefined} */
  export let className = undefined;

  $: classes = makeClassName(["dusk-words", className]);
</script>

<ol {...$$restProps} class={classes}>
  {#each words as word, index (`${word}-${index}`)}
    <li class="dusk-words__word">{word === "" ? "_____" : word}</li>
  {/each}
</ol>
