(function () {
  "use strict";

  angular
    .module('mnXDCR')
    .controller('mnXDCREditDialogController', mnXDCREditDialogController);

  function mnXDCREditDialogController($scope, $uibModalInstance, mnPromiseHelper, mnXDCRService, currentSettings, globalSettings, id, source, mnPools, mnPoolDefault) {
    var vm = this;

    vm.pools = mnPools.export;
    vm.poolDefault = mnPoolDefault.export;

    vm.settings = _.extend({fromBucket: source}, globalSettings.data, currentSettings.data);
    vm.settings.filterSkipRestream = "false";
    vm.createReplication = createReplication;


    function createReplication() {
      var settings = mnXDCRService.removeExcessSettings(vm.settings);
      if (vm.pools.isEnterprise) {
        settings.filterExpression = vm.settings.filterExpression;
        settings.filterSkipRestream = (vm.settings.filterSkipRestream === "true");
      }

      var promise = mnXDCRService.saveReplicationSettings(id, settings);
      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors()
        .closeOnSuccess()
        .broadcast("reloadTasksPoller")
        .showGlobalSuccess("Settings saved successfully!");
    };
  }
})();
