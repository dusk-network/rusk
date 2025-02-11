declare namespace svelteHTML {
  interface HTMLAttributes<T> {
    "on:wizardstepchange"?: (
      event: CustomEvent<{ step: number; stepsCount: number }>
    ) => void;
  }
}

/* Aliases for common w3sper's types */

type AccountBalance = import("@dusk/w3sper").AccountBalance;
type AccountSyncer = import("@dusk/w3sper").AccountSyncer;
type AddressBalance = import("@dusk/w3sper").AddressBalance;
type AddressSyncer = import("@dusk/w3sper").AddressSyncer;
type Bookmark = import("@dusk/w3sper").Bookmark;
type Gas = import("@dusk/w3sper").Gas;
type Network = import("@dusk/w3sper").Network;
type Profile = import("@dusk/w3sper").Profile;
type ProfileGenerator = import("@dusk/w3sper").ProfileGenerator;
type StakeInfo = import("@dusk/w3sper").StakeInfo;
