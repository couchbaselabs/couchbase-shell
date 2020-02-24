(function () {
  "use strict";

  angular.module('mnLostConnection', [
    'mnLostConnectionService',
    'mnHelper'
  ]).config(mnLostConnectionConfig);

  function mnLostConnectionConfig($httpProvider) {
    $httpProvider.interceptors.push(['$q', '$injector', interceptorOfErrConnectionRefused]);
  }

  function interceptorOfErrConnectionRefused($q, $injector) {
    var wantedUrls = {};

    return {
      responseError: function (rejection) {
        if (rejection.status <= 0 && (rejection.xhrStatus == "error")) {
          //rejection caused not by us (e.g. net::ERR_CONNECTION_REFUSED)
          wantedUrls[rejection.config.url] = true;
          $injector
            .get("mnLostConnectionService")
            .activate();
        } else {
          if (wantedUrls[rejection.config.url]) { //in order to avoid cached queries
            wantedUrls = {};
            $injector
              .get("mnLostConnectionService")
              .deactivate();
          }
        }
        return $q.reject(rejection);
      },
      response: function (resp) {
        if (wantedUrls[resp.config.url]) {
          wantedUrls = {};
          $injector
            .get("mnLostConnectionService")
            .deactivate();
        }
        return resp;
      }
    };
  }

})();
