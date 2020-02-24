(function () {
  "use strict";

  angular.module('mnSettingsAutoFailoverService', [
  ]).factory('mnSettingsAutoFailoverService', mnSettingsAutoFailoverServiceFactory);

  function mnSettingsAutoFailoverServiceFactory($http) {
    var mnSettingsAutoFailoverService = {
      resetAutoFailOverCount: resetAutoFailOverCount,
      resetAutoReprovisionCount: resetAutoReprovisionCount,
      getAutoFailoverSettings: getAutoFailoverSettings,
      saveAutoFailoverSettings: saveAutoFailoverSettings,
      getAutoReprovisionSettings: getAutoReprovisionSettings,
      postAutoReprovisionSettings: postAutoReprovisionSettings
    };

    return mnSettingsAutoFailoverService;

    function resetAutoFailOverCount(mnHttpParams) {
      return $http({
        method: 'POST',
        url: '/settings/autoFailover/resetCount',
        mnHttp: mnHttpParams
      });
    }
    function getAutoFailoverSettings() {
      return $http({
        method: 'GET',
        url: "/settings/autoFailover"
      }).then(function (resp) {
        return resp.data;
      });
    }
    function saveAutoFailoverSettings(autoFailoverSettings, params) {
      return $http({
        method: 'POST',
        url: "/settings/autoFailover",
        data: autoFailoverSettings,
        params: params
      });
    }
    function getAutoReprovisionSettings() {
      return $http({
        method: 'GET',
        url: "/settings/autoReprovision"
      });
    }
    function postAutoReprovisionSettings(settings, params) {
      return $http({
        method: 'POST',
        url: "/settings/autoReprovision",
        data: settings,
        params: params
      });
    }
    function resetAutoReprovisionCount(mnHttpParams) {
      return $http({
        method: 'POST',
        url: "/settings/autoReprovision/resetCount",
        mnHttp: mnHttpParams
      });
    }
  }
})();
