(function () {
  "use strict";

  angular.module('mnXDCR', [
    'mnXDCRService',
    'mnHelper',
    'mnPromiseHelper',
    'mnPoll',
    'mnAutocompleteOff',
    'mnPoolDefault',
    'mnPools',
    'mnSortableTable',
    'mnSpinner',
    "ui.codemirror",
    'mnAlertsService'
  ]).controller('mnXDCRController', mnXDCRController);

  function mnXDCRController($scope, permissions, $uibModal, mnHelper, mnPoller, mnPoolDefault, mnXDCRService, mnTasksDetails, mnPromiseHelper) {
    var vm = this;

    vm.mnPoolDefault = mnPoolDefault.latestValue();

    vm.createClusterReference = createClusterReference;
    vm.deleteClusterReference = deleteClusterReference;
    vm.editClusterReference = editClusterReference;
    vm.showReplicationErrors = showReplicationErrors;

    vm.createReplications = createReplications;

    mnHelper.initializeDetailsHashObserver(vm, 'xdcrDetails', 'app.admin.replications');

    activate();

    vm.toBucket = toBucket;
    vm.toCluster = toCluster;
    vm.humanStatus = humanStatus;

    function toBucket(row) {
      return row.target.split('buckets/')[1];
    }
    function toCluster(row) {
      var uuid = row.id.split("/")[0];
      var clusters = vm.references ? vm.references.byUUID : {};
      var toName = !clusters[uuid] ? "unknown" : !clusters[uuid].deleted ? clusters[uuid].name : ('at ' + cluster[uuid].hostname);
      return toName;
    }
    function humanStatus(row) {
      if (row.pauseRequested && row.status != 'paused') {
        return 'pausing';
      } else {
        switch (row.status) {
          case 'running': return 'replicating';
          case 'paused': return 'paused';
          default: return 'starting up';
        }
      }
    }

    function activate() {
      if ($scope.rbac.cluster.xdcr.remote_clusters.read) {
        new mnPoller($scope, function () {
          vm.showReferencesSpinner = false;
          return mnXDCRService.getReplicationState();
        })
        .setInterval(10000)
        .subscribe("references", vm)
        .reloadOnScopeEvent("reloadXdcrPoller", vm, "showReferencesSpinner")
        .cycle();
      }
    }
    function createClusterReference() {
      $uibModal.open({
        controller: 'mnXDCRReferenceDialogController as xdcrReferenceDialogCtl',
        templateUrl: 'app/mn_admin/mn_xdcr_reference_dialog.html',
        scope: $scope,
        resolve: {
          reference: mnHelper.wrapInFunction()
        }
      });
    }
    function deleteClusterReference(row) {
      $uibModal.open({
        controller: 'mnXDCRDeleteReferenceDialogController as xdcrDeleteReferenceDialogCtl',
        templateUrl: 'app/mn_admin/mn_xdcr_delete_reference_dialog.html',
        scope: $scope,
        resolve: {
          name: mnHelper.wrapInFunction(row.name)
        }
      });
    }
    function editClusterReference(reference) {
      $uibModal.open({
        controller: 'mnXDCRReferenceDialogController as xdcrReferenceDialogCtl',
        templateUrl: 'app/mn_admin/mn_xdcr_reference_dialog.html',
        scope: $scope,
        resolve: {
          reference: mnHelper.wrapInFunction(reference)
        }
      });
    }
    function createReplications() {
      $uibModal.open({
        controller: 'mnXDCRCreateDialogController as xdcrCreateDialogCtl',
        templateUrl: 'app/mn_admin/mn_xdcr_create_dialog.html',
        scope: $scope,
        resolve: {
          replicationSettings: mnHelper.wrapInFunction(mnXDCRService.getReplicationSettings())
        }
      });
    }
    function showReplicationErrors(row) {
      vm.xdcrErrors = row.errors;
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_xdcr_errors_dialog.html',
        scope: $scope
      }).result['finally'](function () {
        delete vm.xdcrErrors;
      });
    }
  }
})();
