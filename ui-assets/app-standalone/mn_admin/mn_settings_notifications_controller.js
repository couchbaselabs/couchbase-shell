(function () {
  "use strict";

  angular
    .module('mnSettingsNotifications', [
      'mnSettingsNotificationsService',
      'mnPromiseHelper',
      'mnSettingsClusterService'
    ])
    .controller('mnSettingsNotificationsController', mnSettingsNotificationsController);

  function mnSettingsNotificationsController($scope, mnPromiseHelper, mnSettingsNotificationsService, pools, mnSettingsClusterService) {
    var vm = this;

    mnSettingsClusterService.registerSubmitCallback(submit);
    vm.implementationVersion = pools.implementationVersion;

    activate();

    function activate() {
      mnPromiseHelper(vm, mnSettingsNotificationsService.maybeCheckUpdates())
        .applyToScope("updates");
    }

    function submit() {
      return mnPromiseHelper(vm, mnSettingsNotificationsService.saveSendStatsFlag(vm.updates.enabled))
        .catchGlobalErrors('An error occured, update notifications settings were not saved.')
        .getPromise();
    }
  }
})();
