(function () {
  "use strict";

  angular
    .module('mnLogs')
    .controller('mnLogsCollectInfoController', mnLogsCollectInfoController);

  function mnLogsCollectInfoController($scope, mnHelper, mnPromiseHelper, mnPoolDefault, mnLogsCollectInfoService, mnPoller, $state, $uibModal, mnLogRedactionService, permissions, mnGroupsService, mnLogsService) {
    var vm = this;
    vm.stopCollection = stopCollection;
    vm.isNodeDisabled = isNodeDisabled;
    vm.submit = submit;
    vm.showClusterInfoDialog = mnLogsService.showClusterInfoDialog;

    activate();

    vm.collect = {};
    vm.mnSelectedNodesHolder = {};

    if (mnPoolDefault.latestValue().value.isEnterprise) {
      vm.collect.uploadHost = 'uploads.couchbase.com';
    }

    function isNodeDisabled(node) {
      return node.status === 'unhealthy';
    }

    function activate() {
      if (mnPoolDefault.export.isGroupsAvailable && permissions.cluster.server_groups.read) {
        new mnPoller($scope, mnGroupsService.getGroupsByHostname)
          .subscribe("getGroupsByHostname", vm)
          .cycle();
      }
      new mnPoller($scope, mnLogsCollectInfoService.getState)
        .subscribe(function (state) {
          vm.loadingResult = false;
          vm.state = state;
        })
        .reloadOnScopeEvent("reloadCollectInfoPoller", vm, "loadingResult")
        .reloadOnScopeEvent("mnTasksDetailsChanged")
        .cycle();

      if (permissions.cluster.settings.read &&
          mnPoolDefault.export.compat.atLeast55 &&
          mnPoolDefault.export.isEnterprise) {
        mnLogRedactionService.get().then(function (value) {
          vm.collect.logRedactionLevel = value.logRedactionLevel;
        });
      }
    }

    function stopCollection() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_logs_collect_info_stop_dialog.html'
      }).result.then(function () {
        vm.disabledStopCollect = true;
        mnPromiseHelper(vm, mnLogsCollectInfoService.cancelLogsCollection())
          .getPromise()['finally'](function () {
            vm.disabledStopCollect = false;
          });
      });
    }
    function submit() {
      vm.collect.nodes = mnHelper.checkboxesToList(vm.mnSelectedNodesHolder);
      mnPromiseHelper(vm, mnLogsCollectInfoService.startLogsCollection(vm.collect))
        .showSpinner()
        .catchErrors()
        .onSuccess(function () {
          vm.loadingResult = true;
          $state.go('app.admin.logs.collectInfo.result');
        });
    }
  }
})();
