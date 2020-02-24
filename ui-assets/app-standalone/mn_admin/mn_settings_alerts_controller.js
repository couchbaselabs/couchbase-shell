(function () {
  "use strict";

  angular
    .module('mnSettingsAlerts', [
      'mnSettingsAlertsService',
      'mnHelper',
      'mnPromiseHelper',
      'mnAutocompleteOff'
    ])
    .controller('mnSettingsAlertsController', mnSettingsAlertsController)
    .filter('alertsLabel', alertsLabelFilter);

  function alertsLabelFilter(knownAlerts) {
    return function (name) {
      switch (name) {
        case knownAlerts[0]: return 'Node was auto-failed-over';
        case knownAlerts[1]: return 'Maximum number of auto-failed-over nodes was reached';
        case knownAlerts[2]: return 'Node wasn\'t auto-failed-over as other nodes are down at the same time';
        case knownAlerts[3]: return 'Node was not auto-failed-over as there are not enough nodes in the cluster running the same service';
        case knownAlerts[4]: return 'Node was not auto-failed-over as auto-failover for one or more services running on the node is disabled';
        case knownAlerts[5]: return 'Node\'s IP address has changed unexpectedly';
        case knownAlerts[6]: return 'Disk space used for persistent storage has reached at least 90% of capacity';
        case knownAlerts[7]: return 'Metadata overhead is more than 50%';
        case knownAlerts[8]: return 'Bucket memory on a node is entirely used for metadata';
        case knownAlerts[9]: return 'Writing data to disk for a specific bucket has failed';
        case knownAlerts[10]: return 'Writing event to audit log has failed';
        case knownAlerts[11]: return 'Approaching full Indexer RAM warning';
        case knownAlerts[12]: return 'Remote mutation timestamp exceeded drift threshold';
        case knownAlerts[13]: return 'Communication issues among some nodes in the cluster';
      }
    };
  }

  function mnSettingsAlertsController($scope, mnHelper, mnPromiseHelper, mnSettingsAlertsService) {
    var vm = this;
    vm.isFormElementsDisabled = isFormElementsDisabled;
    vm.testEmail = testEmail;
    vm.submit = submit;
    vm.reloadState = mnHelper.reloadState;

    activate();

    function watchOnAlertsSettings(alertsSettings) {
      if (!$scope.rbac.cluster.settings.write) {
        return;
      }
      mnPromiseHelper(vm, mnSettingsAlertsService.saveAlerts(getParams(), {just_validate: 1}))
        .catchErrorsFromSuccess();
    }
    function submit() {
      var params = getParams();

      mnPromiseHelper(vm, mnSettingsAlertsService.saveAlerts(params))
        .showGlobalSpinner()
        .catchErrors()
        .showGlobalSuccess("Settings saved successfully!")
        .onSuccess(getState);
    }
    function getState() {
      return mnPromiseHelper(vm, mnSettingsAlertsService.getAlerts())
        .onSuccess(function (data) {
          vm.state = data;
          vm.validState = _.merge({},data);
        });
    }
    function activate() {
      getState()
        .onSuccess(function (data) {
          $scope.$watch('settingsAlertsCtl.state', _.debounce(watchOnAlertsSettings, 500, {leading: true}), true);
        });
    }
    function testEmail() {
      var params = getParams();
      params.subject = 'Test email from Couchbase Server';
      params.body = 'This email was sent to you to test the email alert email server settings.';

      mnPromiseHelper(vm, mnSettingsAlertsService.testMail(params))
        .showGlobalSpinner()
        .showGlobalSuccess('Test email was sent successfully!')
        .catchGlobalErrors('An error occurred during sending test email.');
    }
    function isFormElementsDisabled() {
      return !vm.state || !vm.state.enabled;
    }
    function getParams() {
      var params
      if (vm.state.enabled === false) {
        params = _.clone(vm.validState);
        params.enabled = false;
      } else {
        params = _.clone(vm.state);
      }
      params.alerts = mnHelper.checkboxesToList(params.alerts);
      params.recipients = params.recipients.replace(/\s+/g, ',');
      params.emailUser = params.emailServer.user;
      params.emailPass = params.emailServer.pass;
      params.emailHost = params.emailServer.host;
      params.emailPort = params.emailServer.port;
      params.emailEncrypt = params.emailServer.encrypt;
      delete params.emailServer;
      delete params.knownAlerts;
      return params;
    }
  }
})();
