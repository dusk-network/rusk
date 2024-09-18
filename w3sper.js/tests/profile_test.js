// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import {
  test,
  assert,
} from "http://rawcdn.githack.com/mio-mini/test-harness/0.1.0/mod.js";

import { Network, ProfileGenerator } from "../src/mod.js";

// Define a seed for deterministic profile generation
const SEED = new Uint8Array(64);
const seeder = async () => SEED;

// Test case for initial profile generation
test("Initial Profile Generation", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const profiles = new ProfileGenerator(seeder);

  // Verify that the profile list is initially empty
  assert.equal(profiles.length, 0);

  await network.disconnect();
});

// Test case for default profile
test("Default Profile", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const profiles = new ProfileGenerator(seeder);
  const defaultProfile = await profiles.default;

  // Validate the default profile's address and account details
  assert.equal(
    defaultProfile.address.toString(),
    "ivmscertKgRyX8wNMJJsQcSVEyPsfSMUQXSAgeAPQXsndqFq9Pmknzhm61QvcEEdxPaGgxDS4RHpb6KKccrnSKN",
  );

  assert.equal(
    defaultProfile.account.toString(),
    "qe1FbZxf6YaCAeFNSvL1G82cBhG4Q4gBf4vKYo527Vws3b23jdbBuzKSFsdUHnZeBgsTnyNJLkApEpRyJw87sdzR9g9iESJrG5ZgpCs9jq88m6d4qMY5txGpaXskRQmkzE3",
  );

  // Ensure the default profile is indexed correctly
  assert.equal(+defaultProfile, 0);
  assert.equal(profiles.indexOf(defaultProfile), 0);
  assert.equal(await profiles.at(0), defaultProfile);
  assert.equal(await profiles.at(1), undefined);

  // Verify that the profile list has been updated to include the default profile
  assert.equal(profiles.length, 1);

  await network.disconnect();
});

// Test case for generating the next profile
test("Next Profile Generation", async () => {
  const network = await Network.connect("http://localhost:8080/");

  const profiles = new ProfileGenerator(seeder);

  // Generate the next profile
  const profile = await profiles.next();

  // Validate the next profile's address and account details
  assert.equal(
    profile.address.toString(),
    "3MoVQ6VfGNu8fJ5GeHPRDVUfxcsDEmGXpWhvKhXY7F2dKCp7QWRw8RqPcbuJGdRqeTtxpuiwETnGAJLnhT4Kq4e8",
  );

  assert.equal(
    profile.account.toString(),
    "25omWWRyfcMjYNbQZVyc3rypYLi8UqZuthoJHqbCEriRX3z2EmnBaXWZLFL2NvzvDnkoYoHLGiSYQpmupNJj1sSdWNstqzfFEpiqvSNYw7gqvoEiU9FsEHUMG1ZyG3XgL8Rv",
  );

  // Ensure the next profile is indexed correctly
  assert.equal(+profile, 1);
  assert.equal(profiles.indexOf(profile), 1);
  assert.equal(await profiles.at(1), profile);

  // Verify that the profile list has been updated to include the next profile
  assert.equal(profiles.length, 2);

  await network.disconnect();
});

// Test case for ProfileGenerator type checking
test("ProfileGenerator Type Checking", () => {
  // Ensure that non-valid inputs return "undefined"
  assert.equal(ProfileGenerator.typeOf("foobar"), "undefined");
  assert.equal(ProfileGenerator.typeOf("foo bar baz"), "undefined");

  // Ensure that valid addresses and accounts return the correct type
  assert.equal(
    ProfileGenerator.typeOf(
      "2irdhYx8kecxoNEqsNRyVTZnujUztjv2jgVmMMfd84Z2ps2Fa3o5WeDH9xqHT2ebAr66RLvbUsehuCs5UjYXWhkL",
    ),
    "address",
  );

  assert.equal(
    ProfileGenerator.typeOf(
      "2Uq6Zx6RtWLFiQVocJZ7cpD22uVdyyusCYdCQyor4RqLS8DBvAFVWfFXboy6oEvgRzRKFzuv39ftt3HtRk6NZcFboR8PSomkjrpXbTLMEA42AvRAfwe81kyrRuvr8cwhonKj",
    ),
    "account",
  );
});
