<script>
  // eslint-disable-next-line import/default
  import QrScanner from "qr-scanner";
  import { Button } from "$lib/dusk/components";
  import { createEventDispatcher, onDestroy, onMount } from "svelte";

  /** @type {number} */
  let timeoutId;

  /** @type {boolean} */
  let toggleScanner = false;

  /** @type {boolean} */
  let qrScanned = false;

  /** @type {boolean} */
  let error = false;

  /** @type {HTMLVideoElement} */
  let video;

  /** @type {HTMLDivElement} */
  let overlay;

  /** @type {QrScanner | undefined} */
  export let scanner;

  export const startScan = async () => {
    try {
      toggleScanner = true;
      clearTimeout(timeoutId);
      scanner?.start();
    } catch (e) {
      error = !error;
    }
  };

  const dispatch = createEventDispatcher();
  const stopScan = () => {
    toggleScanner = false;
    qrScanned = false;
    error = false;
    scanner?.stop();
  };

  /** @param {QrScanner.ScanResult} result */
  const onDecodedQr = (result) => {
    if (result.data) {
      qrScanned = true;
      dispatch("scan", result.data);
      timeoutId = window.setTimeout(stopScan, 200);
    }
  };

  onMount(async () => {
    const hasCamera = await QrScanner.hasCamera();

    if (hasCamera) {
      scanner = new QrScanner(video, onDecodedQr, {
        highlightScanRegion: true,
        maxScansPerSecond: 1,
        overlay: overlay,
        returnDetailedScanResult: true,
      });
    }
  });

  onDestroy(() => {
    scanner?.destroy();
  });
</script>

<div class="scan" class:scan--visible={toggleScanner}>
  {#if !error}
    <video bind:this={video}>
      <track kind="captions" />
    </video>
    <div
      bind:this={overlay}
      class="scan__highlight"
      class:scan__highlight--scanned={qrScanned}
    ></div>
  {:else}
    <div class="scan__notice">
      <span>An Error occurred while starting camera</span>
    </div>
  {/if}

  <Button size="small" on:click={() => stopScan()} text="CLOSE" />
</div>

<style lang="postcss">
  .scan {
    display: none;
    flex-direction: column;
    max-height: 100%;
    gap: var(--default-gap);
    align-items: center;
    justify-content: center;

    &--visible {
      display: flex;
      position: absolute;
      left: 0px;
      top: 0px;
      background-color: var(--background-color-alt);
      border-radius: 1em;
      padding: 1em;
      width: 100%;
      min-height: 100%;
      z-index: 3;
      video {
        width: 100%;
        margin: auto 0;
        height: 100%;
        overflow: hidden;
      }
    }

    &__highlight {
      border-radius: 1em;
      outline: rgba(0, 0, 0, 0.25) dashed 2px;

      &--scanned {
        outline: #16db93 dashed 2px;
      }
    }
  }
</style>
