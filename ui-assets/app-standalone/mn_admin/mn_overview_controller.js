(function () {
  "use strict";

  angular.module('mnOverview', [
    'mnOverviewService',
    'mnBarUsage',
    'mnPlot',
    'mnBucketsService',
    'mnPoll',
    'ui.bootstrap',
    'mnElementCrane',
    'mnDropdown',
    'mnPromiseHelper',
    'mnXDCRService',
    'mnHelper',
    'mnPoolDefault'
  ]).controller('mnOverviewController', mnOverviewController);

  function mnOverviewController($scope, $rootScope, mnBucketsService, mnOverviewService, mnPoller, mnPromiseHelper, mnHelper, mnXDCRService, permissions, pools, mnPoolDefault) {
    var vm = this;

    vm.getEndings = mnHelper.getEndings;
    vm.addressFamily = mnPoolDefault.export.thisNode.addressFamily;
    vm.nodeEncryption = mnPoolDefault.export.thisNode.nodeEncryption;

    activate();

    function activate() {
      $rootScope.$broadcast("reloadPoolDefaultPoller");

      if (permissions.cluster.xdcr.remote_clusters.read) {
        new mnPoller($scope, mnXDCRService.getReplicationState)
          .setInterval(3000)
          .subscribe("xdcrReferences", vm)
          .cycle();
      }

      new mnPoller($scope, mnOverviewService.getOverviewConfig)
        .reloadOnScopeEvent("mnPoolDefaultChanged")
        .subscribe("mnOverviewConfig", vm)
        .cycle();
      new mnPoller($scope, function () {
          return mnOverviewService.getServices();
        })
        .reloadOnScopeEvent("nodesChanged")
        .subscribe("nodes", vm)
        .cycle();

      if (permissions.cluster.bucket['.'].stats.read) {
        new mnPoller($scope, mnOverviewService.getStats)
          .setInterval(3000)
          .subscribe("mnOverviewStats", vm)
          .cycle();
      }
    }
  }
})();
