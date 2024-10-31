// @ts-nocheck
if (!Promise.withResolvers) {
  Promise.withResolvers = function () {
    let reject;
    let resolve;

    const promise = new Promise((res, rej) => {
      reject = rej;
      resolve = res;
    });

    return { promise, reject, resolve };
  };
}
