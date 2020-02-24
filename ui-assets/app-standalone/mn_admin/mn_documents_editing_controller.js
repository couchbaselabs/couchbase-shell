(function () {
  "use strict";

  angular
    .module("mnDocuments")
    .controller("mnDocumentsEditingController", mnDocumentsEditingController);

  function mnDocumentsEditingController($scope, $state, $uibModal, mnDocumentsEditingService, mnPromiseHelper) {
    var vm = this;
    var codemirrorOptions = {
      lineNumbers: true,
      matchBrackets: false,
      lineWrapping: true,
      tabSize: 2,
      mode: {
        name: "javascript",
        json: true
      },
      theme: 'default'
    };

    var metaOptions = _.clone(codemirrorOptions);
    metaOptions.readOnly = true;
    metaOptions.lineWrapping = true;

    var editorOptions = _.clone(codemirrorOptions);
    editorOptions.onLoad = codemirrorLoaded;

    vm.editorOptions = editorOptions;
    vm.metaOptions = metaOptions;

    vm.isDeleteDisabled = isDeleteDisabled;
    vm.isSaveAsDisabled = isSaveAsDisabled;
    vm.isSaveDisabled = isSaveDisabled;
    vm.isEditorDisabled = isEditorDisabled;
    vm.areThereWarnings = areThereWarnings;
    vm.saveAsDialog = saveAsDialog;
    vm.deleteDocument = deleteDocument;
    vm.save = save;

    function save() {
      var parsedMeta = JSON.parse(vm.state.meta);
      var promise = mnDocumentsEditingService
          .createDocument($state.params, vm.state.doc, parsedMeta.flags)
          .then(function () {
            return mnDocumentsEditingService.getDocumentsEditingState($state.params);
          });
      mnPromiseHelper(vm, promise)
        .showSpinner()
        .catchErrors()
        .applyToScope("state")
        .onSuccess(function (doc) {
          vm.isDocumentChanged = false;
        })
        .showGlobalSuccess("Document saved successfully!");
    }
    function codemirrorLoaded(cm) {
      activate().then(function (resp) {
        if (resp.doc) {
          cm.on("change", function (doc) {
            vm.isDocumentChanged = !documentIsNotChanged(doc.historySize());
            onDocValueUpdate(doc.getValue());
          });
        }
      });
    }
    function deleteDocument(documentId) {
      return $uibModal.open({
        controller: 'mnDocumentsDeleteDialogController as documentsDeleteDialogCtl',
        templateUrl: 'app/mn_admin/mn_documents_delete_dialog.html',
        resolve: {
          documentId: function () {
            return vm.state.title;
          }
        }
      }).result.then(function () {
        $state.go("^.control.list");
      });
    }
    function saveAsDialog() {
      return $uibModal.open({
        controller: 'mnDocumentsCreateDialogController as documentsCreateDialogCtl',
        templateUrl: 'app/mn_admin/mn_documents_create_dialog.html',
        resolve: {
          doc: function () {
            return vm.state.doc;
          }
        }
      }).result.then(function (resp) {
        $state.go("^.editing", {
          documentId: resp.documentId
        });
      })
    }
    function documentIsNotChanged(history) {
      return history.redo == 0 && history.undo == 1;
    }
    function areThereWarnings() {
      return vm.state && _.chain(vm.state.editorWarnings).values().some().value();
    }
    function areThereErrors() {
      return !!vm.state.errors || areThereWarnings();
    }

    function hasDocUpsert() {
      return $scope.rbac.cluster.bucket[$state.params.bucket] &&
        $scope.rbac.cluster.bucket[$state.params.bucket].data.docs.upsert;
    }
    function isEditorDisabled() {
      return !vm.state ||
        (vm.state.editorWarnings &&
         (vm.state.editorWarnings.notFound ||
          vm.state.editorWarnings.documentIsBase64 ||
          vm.state.editorWarnings.documentLimitError)) ||
        !hasDocUpsert();
    }
    function isDeleteDisabled() {
      return !vm.state ||
        vm.viewLoading ||
        (vm.state.editorWarnings && vm.state.editorWarnings.notFound) ||
        !hasDocUpsert();
    }
    function isSaveAsDisabled() {
      return !vm.state ||
        vm.viewLoading ||
        areThereErrors() ||
        !hasDocUpsert();
    }
    function isSaveDisabled() {
      return !vm.state ||
        vm.viewLoading ||
        !vm.isDocumentChanged ||
        areThereErrors() ||
        !hasDocUpsert();
    }
    function onDocValueUpdate(json) {
      vm.state.errors = null;

      try {
        var parsedJSON = JSON.parse(json);
        vm.state.editorWarnings = {
          documentLimitError: mnDocumentsEditingService.isJsonOverLimited(
            JSON.stringify(parsedJSON))
        };
      } catch (error) {
        vm.state.errors = {
          reason: error.message,
          error: "Invalid document"
        };
        return false;
      }

      return areThereWarnings() ? false : parsedJSON;
    }
    function activate() {
      $scope.$watch(isEditorDisabled, function (isDisabled) {
        editorOptions.readOnly = !!isDisabled;
        editorOptions.matchBrackets = !isDisabled;
        vm.editorOptions = editorOptions;
      });
      return mnPromiseHelper(vm, mnDocumentsEditingService.getDocumentsEditingState($state.params))
        .applyToScope("state")
        .getPromise();
    }
  }
})();
