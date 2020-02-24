(function () {
  "use strict";

  angular
    .module("mnDocuments")
    .controller("mnDocumentsDeleteDialogController", mnDocumentsDeleteDialogController);

  function mnDocumentsDeleteDialogController($scope, mnDocumentsEditingService, $state, documentId, $uibModalInstance, mnPromiseHelper) {
    var vm = this;
    vm.onSubmit = onSubmit;

    function onSubmit() {
      var promise = mnDocumentsEditingService.deleteDocument({
        bucket: $state.params.bucket,
        documentId: documentId
      });

      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .broadcast("reloadDocumentsPoller")
        .showGlobalSuccess("Document deleted successfully!");
    }
  }
})();
