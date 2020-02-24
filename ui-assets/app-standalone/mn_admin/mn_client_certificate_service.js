(function () {
  "use strict";

  angular
    .module("mnClientCertificateService", [
      "mnPoolDefault"
    ])
    .factory("mnClientCertificateService", mnClientCertificateFactory);

  function mnClientCertificateFactory($http, $q, mnPoolDefault) {
    var mnClientCertificateService = {
      getClientCertificateSettings: getClientCertificateSettings,
      postClientCertificateSettings: postClientCertificateSettings
    };

    return mnClientCertificateService;

    function getClientCertificateSettings() {
      return $http({
        method: 'GET',
        url: '/settings/clientCertAuth',
      }).then(function (resp) {
        return resp.data;
      });
    }

    function postClientCertificateSettings(data){
      var settings = _.clone(data);
      if (settings.state == 'disable') {
        settings.prefixes = settings.prefixes.filter(function (pref) {
          return !_.isEqual(pref, {delimiter: '', prefix: '', path: ''});
        });
      }
      if (!mnPoolDefault.export.compat.atLeast51) {
        (['delimiter', 'prefix', 'path']).forEach(function (key) {
          if (settings.prefixes[0] && settings.prefixes[0][key]) {
            settings[key] = settings.prefixes[0][key];
          }
        });
        delete settings.prefixes;
      }
      return $http({
        method: 'POST',
        url: '/settings/clientCertAuth',
        mnHttp: {
          isNotForm: mnPoolDefault.export.compat.atLeast51
        },
        data: settings,
      }).then(function (resp) {
        return resp.data;
      });
    }
  }
})();
