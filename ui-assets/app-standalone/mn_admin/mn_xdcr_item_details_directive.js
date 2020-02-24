(function () {
  "use strict";

  angular
    .module('mnXDCR')
    .directive('mnXdcrItemDetails', mnXDCRItemDetails);

  function mnXDCRItemDetails() {
    var mnXDCRItemDetails = {
      restrict: 'E',
      scope: {
        row: "=",
        rbac: "="
      },
      controller: mnXDCRItemDetailsController,
      controllerAs: "xdcrItemDetailsCtl",
      templateUrl: 'app/mn_admin/mn_xdcr_item_details.html'
    };

    return mnXDCRItemDetails;

    function mnXDCRItemDetailsController($scope, mnXDCRService, mnHelper, mnPromiseHelper, $uibModal) {
      var vm = this;

      vm.deleteReplication = deleteReplication;
      vm.editReplication = editReplication;
      vm.status = status;
      vm.pausePlayReplication = pausePlayReplication;

      function pausePlayReplication(row) {
        mnPromiseHelper(vm, mnXDCRService.saveReplicationSettings(row.id, {pauseRequested: row.status !== 'paused'}))
          .broadcast(["reloadTasksPoller"], {doNotShowSpinner: true});
      }

      function editReplication(row) {
        $uibModal.open({
          controller: 'mnXDCREditDialogController as xdcrEditDialogCtl',
          templateUrl: 'app/mn_admin/mn_xdcr_edit_dialog.html',
          scope: $scope,
          resolve: {
            source: mnHelper.wrapInFunction(row.source),
            id: mnHelper.wrapInFunction(row.id),
            currentSettings: mnHelper.wrapInFunction(mnXDCRService.getReplicationSettings(row.id)),
            globalSettings: mnHelper.wrapInFunction(mnXDCRService.getReplicationSettings())
          }
        });
      }

      function deleteReplication(row) {
        $uibModal.open({
          controller: 'mnXDCRDeleteDialogController as xdcrDeleteDialogCtl',
          templateUrl: 'app/mn_admin/mn_xdcr_delete_dialog.html',
          scope: $scope,
          resolve: {
            id: mnHelper.wrapInFunction(row.id)
          }
        });
      }

      function status(row) {
        if (row.pauseRequested && row.status != 'paused') {
          return 'spinner';
        } else {
          switch (row.status) {
          case 'running': return 'pause';
          case 'paused': return 'play';
          default: return 'spinner';
          }
        }
      }

    }
  }
})();
