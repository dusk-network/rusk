declare namespace svelteHTML {
  interface HTMLAttributes<T> {
    "on:wizardstepchange"?: (
      event: CustomEvent<{ step: number; stepsCount: number }>
    ) => void;
  }
}
