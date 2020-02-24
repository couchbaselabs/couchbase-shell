(function () {
  "use strict";

  angular.module('mnXDCR').controller('mnXDCRDeleteReferenceDialogController', mnXDCRDeleteReferenceDialogController);

  function mnXDCRDeleteReferenceDialogController($scope, $uibModalInstance, mnPromiseHelper, mnXDCRService, name) {
    var vm = this;

    vm.name = name;
    vm.deleteClusterReference = deleteClusterReference;

    function deleteClusterReference() {
      var promise = mnXDCRService.deleteClusterReference(name);
      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .catchGlobalErrors()
        .broadcast("reloadXdcrPoller")
        .showGlobalSuccess("Cluster reference deleted successfully!");
    }
  }
})();
