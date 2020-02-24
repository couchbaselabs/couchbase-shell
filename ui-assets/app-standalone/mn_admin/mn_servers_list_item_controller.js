(function () {
  "use strict";

  angular
    .module("mnServers")
    .controller("mnServersListItemController", mnServersListItemController);

  function mnServersListItemController($scope, $state, $rootScope, $uibModal, mnServersService, mnMemoryQuotaService, mnGsiService, mnPromiseHelper, mnPermissions, mnPoolDefault) {

    var vm = this

    vm.cancelEjectServer = cancelEjectServer;
    vm.cancelFailOverNode = cancelFailOverNode;
    vm.reAddNode = reAddNode;
    vm.failOverNode = failOverNode;
    vm.ejectServer = ejectServer;
    vm.disableRemoveBtn = disableRemoveBtn;
    vm.isFailOverDisabled = isFailOverDisabled;

    var ramUsageConf = {};
    var swapUsageConf = {};
    var cpuUsageConf = {};

    activate();

    function activate() {
      $scope.$watch("node", onNodeUpdate, true);
      $scope.$watchGroup(['node', 'adminCtl.tasks'], function (values) {
        vm.getRebalanceProgress = getRebalanceProgress(values[0], values[1]);
      });
    }
    function onNodeUpdate(node) {
      vm.isNodeUnhealthy = isNodeUnhealthy(node);
      vm.isNodeInactiveFailed = isNodeInactiveFailed(node);
      vm.isLastActiveData = isLastActiveData(node);
      vm.isNodeInactiveAdded = isNodeInactiveAdded(node);
      vm.couchDiskUsage = couchDiskUsage(node);

      vm.isKVNode = isKVNode(node);

      vm.getRamUsageConf = getRamUsageConf(node);
      vm.getSwapUsageConf = getSwapUsageConf(node);
      vm.getCpuUsageConf = getCpuUsageConf(node);
    }
    function isKVNode(node) {
      return node.services.indexOf("kv") > -1;
    }
    function getRamUsageConf(node) {
      var total = node.memoryTotal;
      var free = node.memoryFree;
      var used = total - free;

      ramUsageConf.exist = (total > 0) && _.isFinite(free);
      ramUsageConf.value = used / total * 100;

      return ramUsageConf;
    }
    function getSwapUsageConf(node) {
      var swapTotal = node.systemStats.swap_total;
      var swapUsed = node.systemStats.swap_used;
      swapUsageConf.exist = swapTotal > 0 && _.isFinite(swapUsed);
      swapUsageConf.value = (swapUsed / swapTotal) * 100;
      return swapUsageConf;
    }
    function getCpuUsageConf(node) {
      var cpuRate = node.systemStats.cpu_utilization_rate;
      cpuUsageConf.exist = _.isFinite(cpuRate);
      cpuUsageConf.value = Math.floor(cpuRate * 100) / 100;
      return cpuUsageConf;
    }
    function isFailOverDisabled(node) {
      return isLastActiveData(node) || ($scope.adminCtl.tasks && $scope.adminCtl.tasks.inRecoveryMode);
    }
    function disableRemoveBtn(node) {
      return isLastActiveData(node) || isActiveUnhealthy(node) || ($scope.adminCtl.tasks && $scope.adminCtl.tasks.inRecoveryMode);
    }
    function isLastActiveData(node) {
      return $scope.serversCtl.nodes.reallyActiveData.length === 1 && isKVNode(node);
    }
    function isNodeInactiveAdded(node) {
      return node.clusterMembership === 'inactiveAdded';
    }
    function isNodeUnhealthy(node) {
      return node.status === 'unhealthy';
    }
    function isActive(node) {
      return node.clusterMembership === 'active';
    }
    function isNodeInactiveFailed(node) {
      return node.clusterMembership === 'inactiveFailed';
    }
    function couchDiskUsage(node) {
      return node.interestingStats['couch_docs_actual_disk_size'] +
             node.interestingStats['couch_views_actual_disk_size'] +
             node.interestingStats['couch_spatial_disk_size'];
    }
    function getRebalanceProgress(node, tasks) {
      return tasks && (tasks.tasksRebalance.perNode && tasks.tasksRebalance.perNode[node.otpNode]
           ? tasks.tasksRebalance.perNode[node.otpNode].progress : 0 );
    }
    function isActiveUnhealthy(node) {
      return (isActive(node) || isNodeInactiveFailed(node)) && isNodeUnhealthy(node);
    }
    function ejectServer(node) {
      if (isNodeInactiveAdded(node)) {
        mnPromiseHelper(vm, mnServersService.ejectNode({otpNode: node.otpNode}))
          .showErrorsSensitiveSpinner()
          .broadcast("reloadServersPoller");
        return;
      }

      var promise = mnServersService.getNodes().then(function (nodes) {
        var warnings = {
          isLastIndex: mnMemoryQuotaService.isOnlyOneNodeWithService(nodes.allNodes, node.services, 'index', true),
          isLastQuery: mnMemoryQuotaService.isOnlyOneNodeWithService(nodes.allNodes, node.services, 'n1ql', true),
          isLastFts: mnMemoryQuotaService.isOnlyOneNodeWithService(nodes.allNodes, node.services, 'fts', true),
          isLastEventing: mnMemoryQuotaService.isOnlyOneNodeWithService(nodes.allNodes, node.services, 'eventing', true),
          isKv: _.indexOf(node.services, 'kv') > -1
        };
        if (mnPoolDefault.export.isEnterprise) {
          warnings.isLastCBAS = mnMemoryQuotaService.isOnlyOneNodeWithService(nodes.allNodes, node.services, 'cbas', true);
        }
        return mnPermissions.export.cluster.bucket['.'].n1ql.index.read ? mnGsiService.getIndexesState().then(function (indexStatus) {
          warnings.isThereIndex = !!_.find(indexStatus.indexes, function (index) {
            return _.indexOf(index.hosts, node.hostname) > -1;
          });
          warnings.isThereReplica = warnings.isThereIndex;
          return warnings;
        }) : warnings;
      }).then(function (warnings) {
        if (_.some(_.values(warnings))) {
          $uibModal.open({
            templateUrl: 'app/mn_admin/mn_servers_eject_dialog.html',
            controller: 'mnServersEjectDialogController as serversEjectDialogCtl',
            resolve: {
              warnings: function () {
                return warnings;
              },
              node: function () {
                return node;
              }
            }
          });
        } else {
          mnServersService.addToPendingEject(node);
          $rootScope.$broadcast("reloadNodes");
        }
      });

      mnPromiseHelper(vm, promise);
    }
    function failOverNode(node) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_servers_failover_dialog.html',
        controller: 'mnServersFailOverDialogController as serversFailOverDialogCtl',
        resolve: {
          node: function () {
            return node;
          }
        }
      });
    }
    function reAddNode(type, otpNode) {
      mnPromiseHelper(vm, mnServersService.reAddNode({
        otpNode: otpNode,
        recoveryType: type
      }))
      .broadcast("reloadServersPoller")
      .showErrorsSensitiveSpinner();
    }
    function cancelFailOverNode(otpNode) {
      mnPromiseHelper(vm, mnServersService.cancelFailOverNode({
        otpNode: otpNode
      }))
      .broadcast("reloadServersPoller")
      .showErrorsSensitiveSpinner();
    }
    function cancelEjectServer(node) {
      mnServersService.removeFromPendingEject(node);
      $rootScope.$broadcast("reloadNodes");
    }
  }
})();
