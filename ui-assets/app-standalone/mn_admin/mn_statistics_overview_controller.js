(function () {
  "use strict";

  angular
    .module('mnStatisticsNew')
    .controller('mnStatisticsOverviewController', mnStatisticsOverviewController);

function mnStatisticsOverviewController($scope, mnStatisticsNewService, mnStatisticsDescriptionService, $state, $stateParams, $http, mnPoller, mnBucketsService, $uibModal, $rootScope, mnHelper, $window, mnUserRolesService, permissions, $timeout,mnStoreService, mnGsiService, mnViewsListService, mnTasksDetails, mnPermissions) {
    var vm = this;

    vm.mnStatisticsNewScope = $scope;

    vm.rbac = mnPermissions.export;

    vm.node =  $stateParams.overviewHostname;
    vm.bucket = $stateParams.overviewBucket;
    vm.zoom = $stateParams.overviewZoom;

    vm.onSelectNode = onSelectNode;
    vm.onSelectBucket = onSelectBucket;
    vm.onSelectZoom = onSelectZoom;

    vm.getNode = getNode;
    vm.openDetailedChartDialog = openDetailedChartDialog;

    vm.openedState = {"Summary":true};
    vm.myIsDetailsOpened = function(val) {
      return vm.openedState[val];
    }
    vm.myToggleDetails = function(val) {
      vm.openedState[val] = !vm.openedState[val];
    }

    activate();

    function getNode() {
      return vm.node.startsWith("All Server Nodes") ? "all" : vm.node;
    }


    function onSelectNode(selectedHostname) {
      $state.go('.', {overviewHostname: vm.node, overviewBucket: vm.bucket, overviewZoom: vm.zoom}, {reload:true});
    }

    function onSelectBucket(bucket) {
      $state.go('.', {overviewHostname: vm.node, overviewBucket: vm.bucket, overviewZoom: vm.zoom}, {reload:true});
    }

    function onSelectZoom(zoom) {
      $state.go('.', {overviewHostname: vm.node, overviewBucket: vm.bucket, overviewZoom: vm.zoom}, {reload:true});
    }

    function openDetailedChartDialog(block,chart) {
      $state.params.scenarioBucket = vm.bucket;
      $state.params.statsHostname = vm.getNode();
      $uibModal.open(
          {
            templateUrl: 'app/mn_admin/mn_statistics_detailed_chart.html',
            controller: 'mnStatisticsDetailedChartController as detailedChartCtl',
            windowTopClass: "chart-overlay",
            resolve: {
              items: mnHelper.wrapInFunction({}),
              chart: mnHelper.wrapInFunction(chart)
            }
          });
    }

    var fullStatNames = {};
    function extractFullStatNames(stats, prefix) {
      Object.keys(stats).forEach(function(objName) {
        if (objName.startsWith('@'))
          extractFullStatNames(stats[objName],prefix + (prefix.length ? '.' : '') + objName);
        else
          fullStatNames[objName] = prefix + '.' + objName;
      });
    }
    extractFullStatNames(mnStatisticsDescriptionService.stats,'');

    function updateCharts() {
      if (!vm.bucket)
        return;

      mnStatisticsNewService.getStatsDirectory(vm.bucket).then(
      function success(res) {
        vm.blocks = [];
        res.data.blocks.forEach(function(block) {
          var chartBlock = {blockName: block.blockName, charts: [], columns: block.columns};
          block.stats.forEach(function(stat) {
            if (fullStatNames[stat.name]) {
              var chart = {stats: {}, size: "tiny", specificStat: true};
              chart.margin = {top: 10, right: 10, bottom: 16, left: 41};
              chart.stats[fullStatNames[stat.name]] = true;
              chartBlock.charts.push(chart);
            }
          });
          if (chartBlock.charts.length) {
            vm.blocks.push(chartBlock);
          }
        });

      },
      function error(res) {
        console.log("Error getting stats directory: " + JSON.stringify(res));
      });
    }

    function activate() {
      if (!vm.bucket) {
        vm.bucket = vm.rbac.bucketNames['.stats!read'][0];
      }

      if (!vm.zoom) {
        vm.zoom = "hour";
      }

      if ($stateParams.overviewHostname)
        updateCharts();

      new mnPoller($scope, function () {
        return mnStatisticsNewService.prepareNodesList($state.params);
      })
      .subscribe(function (nodes) {
        //nodes.nodesNames.unshift("All Server Nodes (" + nodes.nodesNames.length + ")");
        var origNode = vm.node;
        vm.nodes = nodes;
        if (!vm.node || vm.node == 'all') {
          vm.node = nodes.nodesNames[0];
        }
        if (vm.node != origNode) {
          updateCharts();
        }
      })
      .reloadOnScopeEvent("nodesChanged")
      .cycle();
    }
  }
})();
