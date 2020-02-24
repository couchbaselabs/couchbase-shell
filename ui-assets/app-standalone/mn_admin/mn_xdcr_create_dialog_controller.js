(function () {
  "use strict";

  angular
    .module('mnXDCR')
    .controller('mnXDCRCreateDialogController', mnXDCRCreateDialogController);

  function mnXDCRCreateDialogController($scope, $uibModalInstance, $timeout, $window, mnPromiseHelper, mnPoolDefault, mnPools, mnXDCRService, replicationSettings, mnAlertsService) {
    var vm = this;
    var codemirrorInstance;
    var codemirrorMarkers = [];

    vm.replication = replicationSettings.data;
    delete vm.replication.socketOptions;
    vm.replication.replicationType = "continuous";
    vm.replication.type = "xmem";
    vm.createReplication = createReplication;

    function createReplication() {
      var replication = mnXDCRService.removeExcessSettings(vm.replication);
      if ($scope.poolDefault.isEnterprise) {
        replication.filterExpression = vm.replication.filterExpression;
      }

      var promise = mnXDCRService.postRelication(replication);
      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors(function (error) {
          vm.errors = angular.isString(error) ? {_: error} : error;
        })
        .closeOnSuccess()
        .broadcast("reloadTasksPoller")
        .onSuccess(function (resp) {
          var hasWarnings = !!(resp.data.warnings && resp.data.warnings.length);
          mnAlertsService.formatAndSetAlerts(
            hasWarnings ? resp.data.warnings : "Replication created successfully!",
            hasWarnings ? 'warning': "success",
            hasWarnings ? 0 : 2500);
        });
    };
  }
})();
