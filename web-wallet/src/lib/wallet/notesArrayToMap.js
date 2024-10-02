/** @type {(notes: WalletCacheNote[]) => WalletCacheNotesMap} */
const notesArrayToMap = (notes) => {
  const result = new Map();

  notes.forEach((note) => {
    if (!result.has(note.address)) {
      result.set(note.address, new Map());
    }

    const noteMap = result.get(note.address);

    noteMap.set(note.nullifier, note.note);
  });

  return result;
};

export default notesArrayToMap;
