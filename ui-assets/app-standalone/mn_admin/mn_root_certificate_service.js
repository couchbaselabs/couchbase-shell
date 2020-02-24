(function () {
  "use strict";

  angular
    .module("mnRootCertificateService", [])
    .factory("mnRootCertificateService", mnRootCertificateFactory);

  function mnRootCertificateFactory($http, $q) {
    var mnRootCertificateService = {
      getDefaultCertificate: getDefaultCertificate
    };

    return mnRootCertificateService;

    function getDefaultCertificate() {
      return $http({
        method: 'GET',
        url: '/pools/default/certificate',
        params: {
          extended: true
        }
      }).then(function (resp) {
        return resp.data;
      });
    }
  }
})();
