export default class MediaQueryList {
    /**
     * @param {String} query 
     */
    constructor(query) {
      this.matches = false;
      this.media = query;
      this.listeners = [];
    }
    
    /**
     * @param {String} event 
     * @param {Function} callback 
     */
    addEventListener(event, callback) {
      if (event === 'change') {
        this.listeners.push(callback);
      }
    }
  
    /**
     * @param {String} event 
     * @param {Function} callback 
     */
    removeEventListener(event, callback) {
      if (event === 'change') {
        this.listeners = this.listeners.filter(listener => listener !== callback);
      }
    }
  
    /**
     * @param {Boolean} matches 
     */
    change(matches) {
      this.matches = matches;
      this.listeners.forEach(listener => listener({ matches }));
    }
  }