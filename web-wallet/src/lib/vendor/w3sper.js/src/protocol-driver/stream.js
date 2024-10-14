// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

function createBYOBReadableStream(stream) {
  const reader = stream.getReader();
  let leftoverBytes = null; // Store leftover bytes from a previous read

  return new ReadableStream({
    type: "bytes", // Ensures the stream supports BYOB readers

    async pull(controller) {
      let view = controller.byobRequest?.view;

      if (!view) {
        return;
      }

      let bytesFilled = 0;

      // Helper function to read from the stream and fill the buffer
      function processChunk() {
        // If we have leftover bytes, consume them first
        if (leftoverBytes) {
          const bytesToCopy = Math.min(
            leftoverBytes.byteLength,
            view.byteLength - bytesFilled
          );
          view.set(leftoverBytes.subarray(0, bytesToCopy), bytesFilled);
          bytesFilled += bytesToCopy;

          if (bytesToCopy < leftoverBytes.byteLength) {
            leftoverBytes = leftoverBytes.subarray(bytesToCopy); // Store remaining leftovers
          } else {
            leftoverBytes = null; // All leftover bytes are consumed
          }

          if (bytesFilled >= view.byteLength) {
            controller.byobRequest.respond(bytesFilled); // Respond with the filled buffer
            return; // Done filling the buffer
          }
        }

        // Read from the underlying stream if more data is needed
        reader.read().then(({ done, value }) => {
          if (done) {
            if (bytesFilled === 0) {
              controller.close();
              controller.byobRequest.respond(0);
            } else {
              // Respond with whatever we have, then close the stream
              controller.byobRequest.respond(bytesFilled);
              controller.close();
            }

            return;
          }

          // Copy the new chunk into the buffer
          const bytesToCopy = Math.min(
            value.byteLength,
            view.byteLength - bytesFilled
          );
          view.set(value.subarray(0, bytesToCopy), bytesFilled);
          bytesFilled += bytesToCopy;

          if (bytesToCopy < value.byteLength) {
            // Store leftover bytes for the next pull
            leftoverBytes = value.subarray(bytesToCopy);
          }

          // If the buffer is still not full, keep reading
          if (bytesFilled < view.byteLength) {
            processChunk(); // Recursively read more data
          } else {
            controller.byobRequest.respond(bytesFilled); // Buffer is fully filled
          }
        });
      }

      // Start filling the buffer
      processChunk();
    },
    cancel() {
      // Close the reader when the stream is canceled
      reader.cancel();
    },
  });
}

export function getBYOBReader(stream) {
  const r = createBYOBReadableStream(stream);
  return r.getReader({ mode: "byob" });
}
