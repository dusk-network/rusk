/** @type {(delay: number, value: any) => Promise<typeof value>} */
const resolveAfter = (delay, value) =>
  new Promise((resolve) => {
    setTimeout(() => resolve(value), delay);
  });

export default resolveAfter;
