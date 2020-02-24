(function () {
  "use strict";

  angular
    .module('mnSettingsAlertsService', [
      "mnHelper"
    ])
    .factory('mnSettingsAlertsService', mnSettingsAlertsService);

  function mnSettingsAlertsService($http, knownAlerts, mnHelper) {
    var mnSettingsAlertsService = {
      testMail: testMail,
      saveAlerts: saveAlerts,
      getAlerts: getAlerts
    };

    return mnSettingsAlertsService;

    function testMail(params) {
      params = _.clone(params);
      params.alerts = params.alerts.join(',');
      return $http.post('/settings/alerts/testEmail', params);
    }
    function saveAlerts(settings, params) {
      settings = _.clone(settings);
      settings.alerts = settings.alerts.join(',');
      return $http.post('/settings/alerts', settings, {params: params});
    }
    function getAlerts() {
      return $http.get('/settings/alerts').then(function (resp) {
        var val = _.clone(resp.data);
        val.recipients = val.recipients.join('\n');
        val.knownAlerts = _.clone(knownAlerts);
        val.alerts = mnHelper.listToCheckboxes(val.alerts);

        return val;
      });
    }
  }
})();
