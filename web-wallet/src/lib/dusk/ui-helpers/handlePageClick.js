/**
 * Action to handle clicks outside a specified node.
 *
 * @param {Node} node - The DOM node to monitor for outside clicks.
 * @param {Object} config - Configuration object for the action.
 * @param {boolean} config.enabled - Whether the outside click listener is active.
 * @param {Function} config.callback - Callback to execute when an outside click is detected.
 */
export function handlePageClick(node, { enabled: initialEnabled, callback }) {
  // @ts-ignore
  const handleClick = (event) => {
    if (node && !node.contains(event.target) && !event.defaultPrevented) {
      callback();
    }
  };

  // @ts-ignore
  function update({ enabled }) {
    if (enabled) {
      window.addEventListener("click", handleClick);
    } else {
      window.removeEventListener("click", handleClick);
    }
  }

  update({ enabled: initialEnabled });

  return {
    destroy() {
      window.removeEventListener("click", handleClick);
    },
    update,
  };
}
