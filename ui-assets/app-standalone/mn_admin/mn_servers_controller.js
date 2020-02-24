(function () {
  "use strict";

  angular
    .module('mnServers', [
      'mnPoolDefault',
      'ui.router',
      'mnAutocompleteOff',
      'ui.bootstrap',
      'mnServersService',
      'mnHelper',
      'mnBarUsage',
      'mnServersListItemDetailsService',
      'mnFilters',
      'mnSortableTable',
      'mnServices',
      'mnSpinner',
      'ngMessages',
      'mnMemoryQuotaService',
      'mnGsiService',
      'mnPromiseHelper',
      'mnGroupsService',
      'mnStorageMode',
      'mnPoll',
      'mnFocus',
      'mnPools',
      'mnWarmupProgress',
      'mnElementCrane',
      'mnSearch',
      'mnSelectableNodesList',
      'mnRootCertificateService'
    ])
    .controller('mnServersController', mnServersController);

  function mnServersController($scope, $state, $uibModal, mnPoolDefault, mnPoller, mnServersService, mnHelper, mnGroupsService, mnPromiseHelper, mnPools, permissions, mnStatisticsNewService) {
    var vm = this;
    vm.mnPoolDefault = mnPoolDefault.latestValue();

    vm.postStopRebalance = postStopRebalance;
    vm.onStopRecovery = onStopRecovery;
    vm.postRebalance = postRebalance;
    vm.addServer = addServer;
    vm.filterField = "";
    vm.sortByGroup = sortByGroup;
    vm.multipleFailoverDialog = multipleFailoverDialog;

    function sortByGroup(node) {
      return vm.getGroupsByHostname[node.hostname] && vm.getGroupsByHostname[node.hostname].name;
    }

    activate();

    function activate() {
      mnHelper.initializeDetailsHashObserver(vm, 'openedServers', 'app.admin.servers.list');

      mnStatisticsNewService.heartbeat.setInterval(function (resp) {
        return resp.interval || 5000;
      });

      if (permissions.cluster.server_groups.read) {
        new mnPoller($scope, function () {
          return mnGroupsService.getGroupsByHostname();
        })
          .subscribe("getGroupsByHostname", vm)
          .reloadOnScopeEvent(["serverGroupsUriChanged", "reloadServersPoller"])
          .cycle();
      }

      new mnPoller($scope, function () {
        return mnServersService.getNodes();
      })
        .subscribe(function (nodes) {
          vm.showSpinner = false;
          vm.nodes = nodes;
        })
        .reloadOnScopeEvent(["mnPoolDefaultChanged", "reloadNodes"])
        .cycle();

      // $scope.$on("reloadServersPoller", function () {
      //   vm.showSpinner = true;
      // });
    }
    function multipleFailoverDialog() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_multiple_failover_dialog.html',
        controller: 'mnMultipleFailoverDialogController as multipleFailoverDialogCtl',
        resolve: {
          groups: function () {
            return mnPoolDefault.get().then(function (poolDefault) {
              if (poolDefault.isGroupsAvailable && permissions.cluster.server_groups.read) {
                return mnGroupsService.getGroupsByHostname();
              }
            });
          },
          nodes: function () {
            return vm.nodes.reallyActive;
          }
        }
      });
    }
    function addServer() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_servers_add_dialog.html',
        controller: 'mnServersAddDialogController as serversAddDialogCtl',
        resolve: {
          groups: function () {
            return mnPoolDefault.get().then(function (poolDefault) {
              if (poolDefault.isGroupsAvailable) {
                return mnGroupsService.getGroups();
              }
            });
          }
        }
      });
    }
    function postRebalance() {
      mnPromiseHelper(vm, mnServersService.postRebalance(vm.nodes.allNodes))
        .onSuccess(function () {
          $state.go('app.admin.servers.list', {list: 'active'});
        })
        .broadcast("reloadServersPoller")
        .catchGlobalErrors()
        .showErrorsSensitiveSpinner();
    }
    function onStopRecovery() {
      mnPromiseHelper(vm, mnServersService.stopRecovery($scope.adminCtl.tasks.tasksRecovery.stopURI))
        .broadcast("reloadServersPoller")
        .showErrorsSensitiveSpinner();
    }
    function postStopRebalance() {
      return mnPromiseHelper(vm, mnServersService.stopRebalanceWithConfirm())
        .broadcast("reloadServersPoller");
    }
  }
})();
