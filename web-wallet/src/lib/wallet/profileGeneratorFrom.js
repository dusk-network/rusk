import { ProfileGenerator } from "$lib/../../../w3sper.js/src/mod";

/** @type {(seed: Uint8Array) => ProfileGenerator} */
function profileGeneratorFrom(seed) {
  /*
   * For now we create a function that returns
   * a constant value.
   * In future we can add some encrypt / decrypt logic.
   */
  const seeder = async () => seed;

  return new ProfileGenerator(seeder);
}

// TODO consider the possibility to pass the mnemonic instead of the seed
export default profileGeneratorFrom;
