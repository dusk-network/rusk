import { writable } from "svelte/store";

/** @param {Number} stepsCount */
function createWizardStore(stepsCount) {
  const initialState = {
    currentStep: 0,
    stepsCount,
  };

  const { subscribe, update } = writable(initialState);

  function incrementStep() {
    update((state) => {
      if (state.currentStep < state.stepsCount - 1) {
        return { ...state, currentStep: state.currentStep + 1 };
      }

      return state;
    });
  }

  function decrementStep() {
    update((state) => {
      if (state.currentStep > 0) {
        return { ...state, currentStep: state.currentStep - 1 };
      }

      return state;
    });
  }

  return {
    decrementStep,
    incrementStep,
    subscribe,
  };
}

export default createWizardStore;
