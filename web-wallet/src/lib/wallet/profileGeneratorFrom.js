import { ProfileGenerator } from "$lib/vendor/w3sper.js/src/mod";

import decryptBuffer from "./decryptBuffer";
import encryptBuffer from "./encryptBuffer";

/** @type {(seed: Uint8Array) => Promise<ProfileGenerator>} */
async function profileGeneratorFrom(seed) {
  // creating a local copy
  seed = seed.slice();

  const pwd = new TextDecoder().decode(
    crypto.getRandomValues(new Uint8Array(32))
  );
  const encryptInfo = await encryptBuffer(seed, pwd);

  // destroying data inside the local copy
  seed.fill(0);

  const seeder = async () =>
    new Uint8Array(await decryptBuffer(encryptInfo, pwd));

  return new ProfileGenerator(seeder);
}

export default profileGeneratorFrom;
