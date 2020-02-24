(function () {
  "use strict";

  angular
    .module("mnLogRedactionService", [])
    .factory("mnLogRedactionService", mnRedactionFactory);

  function mnRedactionFactory($http) {
    var mnLogRedactionService = {
      get: get,
      post: post
    };

    return mnLogRedactionService;

    function get() {
      return $http.get("/settings/logRedaction").then(function (resp) {
        return resp.data;
      });
    }

    function post(data) {
      return $http.post("/settings/logRedaction", data);
    }
  }
})();
