<script>
  import { mdiAlertOutline, mdiContentPaste, mdiRedoVariant } from "@mdi/js";
  import { wordlists } from "bip39";
  import { Button, Textbox, Words } from "$lib/dusk/components";
  import { makeClassName } from "$lib/dusk/string";
  import { findFirstNMatches } from "$lib/dusk/array";
  import { toast } from "$lib/dusk/components/Toast/store";

  /** @type {MnemonicType} */
  export let type;

  /** @type {string|undefined} */
  export let className = undefined;

  /** @type {number} */
  export let wordLimit = 12;

  /** @type {string[]} */
  export let mnemonicPhrase = [];

  /** @type {string[]} */
  export let enteredMnemonicPhrase = [];

  /** @type {number} */
  let currentIndex = 0;

  /** @type {string} */
  let currentInput = "";

  /** @type {Textbox} */
  let textboxElement;

  const classes = makeClassName(["dusk-mnemonic", className]);
  const enteredWordIndex = Array(wordLimit).fill("");
  const enDictionary = wordlists.english;

  if (enteredMnemonicPhrase.length === 0) {
    enteredMnemonicPhrase = Array(wordLimit).fill("");
  }

  const isTriggeredByLogin =
    enteredMnemonicPhrase.some((word) => word !== "") && currentIndex === 0;

  /**
   * @param {string} word
   * @param {string} index
   */
  function updateEnteredPhrase(word, index) {
    enteredMnemonicPhrase[currentIndex] = word;
    enteredWordIndex[currentIndex] = index;
    currentInput = "";
    currentIndex++;

    if (type === "authenticate") {
      focusWordTextboxInput();
    }
  }

  /**
   * Prevents non-alphabetical characters from being entered
   * and auto-selects the first suggestion on Enter
   * if there is only one word available
   * @param {KeyboardEvent} event
   * @param {string} index
   */
  function handleKeyDownOnAuthenticateTextbox(event, index) {
    const isAlphabetical = /^[a-zA-Z]+$/;

    if (!isAlphabetical.test(event.key)) {
      event.preventDefault();
      toast(
        "error",
        "Only alphabetical characters are allowed",
        mdiAlertOutline
      );
      return;
    }

    if (event.key === "Enter" && suggestions[0] === currentInput) {
      updateEnteredPhrase(suggestions[0], index);
    }
  }

  // @ts-ignore
  function handleWordButtonClick(event, index) {
    updateEnteredPhrase(event.currentTarget.dataset.value, index);
  }

  function focusWordTextboxInput() {
    textboxElement?.focus();
  }

  function undoLastWord() {
    if (currentIndex === 0) {
      return;
    }

    currentIndex--;
    enteredMnemonicPhrase[currentIndex] = "";
    enteredWordIndex[currentIndex] = "";
  }

  $: suggestions =
    currentInput &&
    findFirstNMatches(enDictionary, currentInput.toLowerCase(), 3);

  const pasteMnemonic = () => {
    navigator.clipboard
      .readText()
      .then((data) => {
        const sanitizedData = data.replace(/[^a-zA-Z\s]/g, "").toLowerCase();
        const words = sanitizedData.trim().split(/\s+/);

        if (words.length !== 12) {
          throw Error("Mnemonic phrase is not valid");
        }

        currentIndex = 0;
        words.forEach((word) => {
          updateEnteredPhrase(word, currentIndex.toString());
        });
      })
      .catch((err) => {
        if (err.name === "NotAllowedError") {
          toast("error", "Clipboard access denied", mdiAlertOutline);
        } else {
          toast("error", err.message, mdiAlertOutline);
        }
      });
  };

  const shouldShowPaste =
    "clipboard" in navigator &&
    typeof navigator.clipboard.readText === "function";
</script>

<div {...$$restProps} class={classes}>
  {#if !isTriggeredByLogin}
    <div class="dusk-mnemonic__actions-wrapper">
      {#if type === "authenticate" && shouldShowPaste}
        <Button
          icon={{ path: mdiContentPaste }}
          text="Paste mnemonic phrase"
          variant="tertiary"
          on:click={pasteMnemonic}
        />
      {/if}
      <Button
        disabled={!currentIndex}
        on:click={undoLastWord}
        icon={{ path: mdiRedoVariant }}
        text="Undo"
        variant="tertiary"
      />
    </div>
  {/if}

  <Words words={enteredMnemonicPhrase} />

  {#if !isTriggeredByLogin}
    <div
      class={type === "authenticate"
        ? "dusk-mnemonic__authenticate-actions-wrapper"
        : "dusk-mnemonic__validate-actions-wrapper"}
    >
      {#if type === "authenticate" && enteredWordIndex.includes("")}
        <Textbox
          placeholder={`Enter word ${currentIndex + 1}`}
          bind:this={textboxElement}
          on:keydown={(e) =>
            handleKeyDownOnAuthenticateTextbox(e, currentIndex.toString())}
          maxlength={8}
          type="text"
          bind:value={currentInput}
        />
        {#if suggestions.length}
          <div class="dusk-mnemonic__suggestions-wrapper">
            {#each suggestions as suggestion, index (`${suggestion}-${index}`)}
              <Button
                variant="tertiary"
                text={suggestion}
                data-value={suggestion}
                on:click={handleWordButtonClick}
                tabindex="0"
              />
            {/each}
          </div>
        {/if}
      {:else}
        {#each mnemonicPhrase as word, index (`${word}-${index}`)}
          <Button
            variant="tertiary"
            text={word}
            data-value={word}
            disabled={enteredWordIndex.includes(index.toString())}
            on:click={(e) => handleWordButtonClick(e, index.toString())}
          />
        {/each}
      {/if}
    </div>
  {/if}
</div>
