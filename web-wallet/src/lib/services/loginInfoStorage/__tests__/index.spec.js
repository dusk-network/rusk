import { afterEach, describe, expect, it } from "vitest";
import { isNull, mapValuesWith, unless } from "lamb";

import { bytesToBase64 } from "$lib/dusk/base64";

import loginInfoStorage from "..";

describe("loginInfoStorage", () => {
  const storeKey = `${CONFIG.LOCAL_STORAGE_APP_KEY}-login`;
  const valuesToArray = unless(
    isNull,
    mapValuesWith((v) => [...v])
  );
  const valuesToBase64 = mapValuesWith(bytesToBase64);
  const loginInfo = {
    data: new TextEncoder().encode("some string"),
    iv: Uint8Array.of(1, 2, 3, 4),
    salt: Uint8Array.of(5, 6, 7, 8),
  };
  const storedInfo = JSON.stringify(valuesToBase64(loginInfo));

  afterEach(() => {
    localStorage.clear();
  });

  it("should expose a method to retrieve the login info from local storage and convert back its values to Uint8Array", () => {
    localStorage.setItem(storeKey, storedInfo);

    const result = loginInfoStorage.get();

    expect(result).toMatchObject({
      data: expect.any(Uint8Array),
      iv: expect.any(Uint8Array),
      salt: expect.any(Uint8Array),
    });

    // The `toStrictEqual` matcher doesn't play well with typed arrays in this case
    expect(valuesToArray(result)).toStrictEqual(valuesToArray(loginInfo));
  });

  it("should return `null` if there is no login info stored", () => {
    expect(loginInfoStorage.get()).toBeNull();
  });

  it("should expose a method to remove the login info from the local storage", () => {
    localStorage.setItem(storeKey, storedInfo);
    loginInfoStorage.remove();

    expect(localStorage.getItem(storeKey)).toBeNull();
  });

  it("should expose a method to set the login info and convert its values to base64 before serialization", () => {
    loginInfoStorage.set(loginInfo);

    const stored = localStorage.getItem(storeKey);

    expect(stored).toBe(storedInfo);
  });
});
