(function () {
  "use strict";

  angular
    .module('mnServers')
    .controller('mnServersMemoryQuotaDialogController', mnServersMemoryQuotaDialogController);

  function mnServersMemoryQuotaDialogController($scope, indexSettings, $q, mnPoolDefault, $uibModalInstance, mnSettingsClusterService, memoryQuotaConfig, mnPromiseHelper, firstTimeAddedServices) {
    var vm = this;
    vm.config = memoryQuotaConfig;
    vm.isEnterprise = mnPoolDefault.latestValue().value.isEnterprise;
    vm.onSubmit = onSubmit;
    vm.initialIndexSettings = _.clone(indexSettings);
    vm.indexSettings = indexSettings;
    vm.firstTimeAddedServices = firstTimeAddedServices;
    vm.getFirstTimeServiceNames = getFirstTimeServiceNames;

    if (indexSettings.storageMode === "") {
      vm.indexSettings.storageMode = vm.isEnterprise ? "plasma" : "forestdb";
    }

    function onSubmit() {
      if (vm.viewLoading) {
        return;
      }

      var queries = [
        mnPromiseHelper(vm, mnSettingsClusterService.postPoolsDefault(vm.config))
          .catchErrors()
          .getPromise()
      ];

      if (vm.firstTimeAddedServices.index) {
        queries.push(
          mnPromiseHelper(vm, mnSettingsClusterService.postIndexSettings(vm.indexSettings))
            .catchErrors("postIndexSettingsErrors")
            .getPromise()
        );
      }
      var promise = $q.all(queries);

      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .closeOnSuccess()
        .showGlobalSuccess("Memory quota saved successfully!");
    }

    function getFirstTimeServiceNames() {
      var services = [];
      if (firstTimeAddedServices.index)
        services.push("GSI Index");
      if (firstTimeAddedServices.fts)
        services.push("Full Text");
      if (firstTimeAddedServices.cbas)
        services.push("Analytics");
      if (firstTimeAddedServices.eventing)
        services.push("Eventing");

      return services;
    }
  }
})();
