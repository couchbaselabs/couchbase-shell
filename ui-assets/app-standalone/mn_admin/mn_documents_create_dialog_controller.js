(function () {
  "use strict";

  angular
    .module("mnDocuments")
    .controller("mnDocumentsCreateDialogController", mnDocumentsCreateDialogController);

  function mnDocumentsCreateDialogController($scope, mnDocumentsEditingService, mnPromiseHelper, $state, $uibModalInstance, doc) {
    var vm = this;
    vm.onSubmit = onSubmit;

    function onSubmit() {
      var newDocumentParams = {
        bucket: $state.params.bucket,
        documentId: vm.documentId
      };
      var promise = mnDocumentsEditingService.getDocument(newDocumentParams)
        .then(function () {
          vm.error = "Document with given ID already exists";
        }, function (resp) {
          if (resp.status == 400) {
            // Expect the REST API to tell you why this was a bad request.
            vm.error = resp.data && resp.data.reason;
          } else if (resp.status > 400 && resp.status < 500) {
            return mnPromiseHelper(vm, mnDocumentsEditingService.createDocument(newDocumentParams, doc), $uibModalInstance)
              .catchErrors(function (data) {
                vm.error = data && data.reason;
              })
              .closeOnSuccess()
              .onSuccess(function () {
                $state.go('^.^.editing', {
                  documentId: newDocumentParams.documentId
                });
              });
          } else {
            vm.error = resp.data && resp.data.reason;
          }
        });
      mnPromiseHelper($scope, promise, $uibModalInstance)
        .showGlobalSpinner()
        .showGlobalSuccess("Document created successfully!");
    }
  }
})();
