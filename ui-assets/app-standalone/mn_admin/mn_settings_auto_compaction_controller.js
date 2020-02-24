(function () {
  "use strict";

  angular.module('mnSettingsAutoCompaction', [
    'mnSettingsAutoCompactionService',
    'mnHelper',
    'mnPromiseHelper',
    'mnAutoCompactionForm'
  ]).controller('mnSettingsAutoCompactionController', mnSettingsAutoCompactionController);

  function mnSettingsAutoCompactionController($scope, mnHelper, mnPromiseHelper, mnSettingsAutoCompactionService) {
    var vm = this;

    vm.reloadState = mnHelper.reloadState;
    vm.submit = submit;

    activate();

    function activate() {
      mnPromiseHelper(vm, mnSettingsAutoCompactionService.getAutoCompaction())
        .applyToScope("autoCompactionSettings")
        .onSuccess(function () {
          $scope.$watch('settingsAutoCompactionCtl.autoCompactionSettings', watchOnAutoCompactionSettings, true);
        });
    }
    function watchOnAutoCompactionSettings(autoCompactionSettings) {
      if (!$scope.rbac.cluster.settings.write) {
        return;
      }
      mnPromiseHelper(vm, mnSettingsAutoCompactionService
        .saveAutoCompaction(autoCompactionSettings, {just_validate: 1}))
          .catchErrors();
    }
    function submit() {
      if (vm.viewLoading) {
        return;
      }
      delete vm.errors;
      mnPromiseHelper(vm, mnSettingsAutoCompactionService.saveAutoCompaction(vm.autoCompactionSettings))
        .showGlobalSpinner()
        .reloadState()
        .catchErrors()
        .showGlobalSuccess("Settings saved successfully!");
    }
  }
})();
