(function () {
  "use strict";

  angular
    .module('mnStoreService', ["mnHelper"])
    .factory('mnStoreService', mnStoreServiceFactory);

  function mnStoreServiceFactory(mnHelper) {
    var db = {};
    var stores = {};
    var storeService = {
      createStore: createStore,
      store: store
    };

    Store.prototype.get = get;
    Store.prototype.add = add;
    Store.prototype.put = put;
    Store.prototype.delete = _delete;
    Store.prototype.deleteItem = deleteItem;
    Store.prototype.getByIncludes = getByIncludes;
    Store.prototype.share = share;
    Store.prototype.copy = copy;
    Store.prototype.last = last;
    Store.prototype.clear = clear;

    return storeService;

    function store(name) {
      return stores[name];
    }

    function createStore(name, options) {
      stores[name] = new Store(name, options);
    }

    function Store(name, options) {
      this.keyPath = options.keyPath;
      this.name = name;
      if (options.fill) {
        if (db[this.name]) {
          this.clear();
          Array.prototype.push.apply(db[this.name], options.fill);
        } else {
          db[this.name] = options.fill;
        }
      } else {
        db[this.name] = [];
      }
    }

    function last() {
      return db[this.name][db[this.name].length - 1];
    }

    function clear() {
      db[this.name].splice(0, db[this.name].length);
    }

    function share() {
      return db[this.name];
    }

    function copy() {
      return db[this.name].slice();
    }

    function put(item) {
      var copyTo = this.get(item[this.keyPath]);
      Object.keys(item).forEach(function (key) {
        copyTo[key] = item[key];
      });
    }

    function add(item) {
      item = Object.assign({}, item);
      item[this.keyPath] = mnHelper.generateID();
      db[this.name].push(item);
      return item;
    }

    function _delete(value) {
      db[this.name].splice(db[this.name].findIndex(function (item) {
        return item[this.keyPath] == value;
      }.bind(this)), 1);
    }

    function deleteItem(item) {
      db[this.name].splice(db[this.name].indexOf(item), 1);
    }

    function get(value) {
      return db[this.name].find(function (item) {
        return item[this.keyPath] == value;
      }.bind(this));
    }

    function getByIncludes(value, row) {
      return db[this.name].find(function (item) {
        return item[row].includes(value);
      }.bind(this));
    }
  }

})();
