/** @type {(delay: number) => Promise<any>} */
const rejectAfter = (delay) =>
  new Promise((_, reject) => {
    setTimeout(() => reject(new Error("some error")), delay);
  });

export default rejectAfter;
