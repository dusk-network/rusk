import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

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

export default profileGeneratorFrom;
