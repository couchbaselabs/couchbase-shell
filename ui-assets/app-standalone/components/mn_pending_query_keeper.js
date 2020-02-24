(function () {
  "use strict";

  angular
    .module("mnPendingQueryKeeper", [])
    .factory("mnPendingQueryKeeper", mnPendingQueryKeeperFactory);

  function mnPendingQueryKeeperFactory() {
    var pendingQueryKeeper = [];

    return {
      getQueryInFly: getQueryInFly,
      removeQueryInFly: removeQueryInFly,
      push: push,
      cancelTabsSpecificQueries: cancelTabsSpecificQueries,
      cancelAllQueries: cancelAllQueries
    };

    function cancelAllQueries() {
      var i = pendingQueryKeeper.length;
      while (i--) {
        pendingQueryKeeper[i].canceler();
      }
    }
    function cancelTabsSpecificQueries() {
      var i = pendingQueryKeeper.length;
      while (i--) {
        if (pendingQueryKeeper[i].group !== "global") {
          pendingQueryKeeper[i].canceler();
        }
      }
    }

    function removeQueryInFly(findMe) {
      var i = pendingQueryKeeper.length;
      while (i--) {
        if (pendingQueryKeeper[i] === findMe) {
          pendingQueryKeeper.splice(i, 1);
        }
      }
    }

    function getQueryInFly(config) {
      return _.find(pendingQueryKeeper, function (inFly) {
        return inFly.config.method === config.method &&
               inFly.config.url === config.url;
      });
    }

    function push(query) {
      pendingQueryKeeper.push(query);
    }
  }
})();
