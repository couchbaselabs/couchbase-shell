(function () {
  "use strict";

  angular
    .module('mnStatisticsNew', [
      'mnStatisticsNewService',
      'mnStatisticsDescriptionService',
      'mnPoll',
      'mnBucketsService',
      'mnHelper',
      'ui.router',
      'ui.bootstrap',
      'mnBucketsStats',
      'mnSpinner',
      'mnStatisticsChart',
      'mnUserRolesService',
      'mnFilters',
      'mnStoreService'
    ])
    .controller('mnStatisticsNewController', mnStatisticsNewController)
    .controller('mnStatisticsGroupsController', mnStatisticsGroupsController)
    .controller('mnStatisticsChartsController', mnStatisticsChartsController);

  function mnStatisticsChartsController($scope, $rootScope, $uibModal, mnStatisticsNewService, mnStoreService, mnHelper, mnUserRolesService, $state, $timeout) {
    var vm = this;

    vm.deleteChart = deleteChart;
    vm.editChart = editChart;
    vm.openDetailedChartDialog = openDetailedChartDialog;
    vm.chart = mnStoreService.store("charts").get($scope.chartID);
    vm.api = {};

    function deleteChart() {
      vm.showChartControls = false;
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_chart_builder_delete.html'
      }).result.then(function () {
        mnStatisticsNewService.deleteChart($scope.chartID);
        mnUserRolesService.saveDashboard();
        $scope.mnStatsGroupsCtl.maybeShowItemsControls();
      });
    }

    function editChart(group, scenario) {
      vm.showChartControls = false;
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_chart_builder.html',
        controller: 'mnStatisticsNewChartBuilderController as builderCtl',
        resolve: {
          chart: mnHelper.wrapInFunction(vm.chart),
          group: mnHelper.wrapInFunction(group),
          scenario: mnHelper.wrapInFunction(scenario)
        }
      }).result.then(function () {
        mnUserRolesService.saveDashboard();
        vm.reloadChartDirective = true;
        $timeout(function () {
          vm.reloadChartDirective = false;
          $scope.mnStatsGroupsCtl.maybeShowItemsControls();
        });
      });
    }

    function openDetailedChartDialog() {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_detailed_chart.html',
        controller: 'mnStatisticsDetailedChartController as detailedChartCtl',
        windowTopClass: "chart-overlay",
        resolve: {
          items: mnHelper.wrapInFunction($scope.mnStatsGroupsCtl.items),
          chart: mnHelper.wrapInFunction(vm.chart)
        }
      });
    }
  }

  function mnStatisticsGroupsController($scope, $uibModal, $timeout,
                                        mnStatisticsNewService, mnStoreService, mnUserRolesService) {
    var vm = this;
    vm.isDetailsOpened = true;
    vm.hideGroupControls = hideGroupControls;
    vm.onGroupNameBlur = onGroupNameBlur;
    vm.onGroupFocus = onGroupFocus;
    vm.onGroupSubmit = onGroupSubmit;
    vm.onGroupDelete = onGroupDelete;
    vm.deleteGroup = deleteGroup;
    vm.maybeShowItemsControls = maybeShowItemsControls;
    vm.saveDashboard = mnUserRolesService.saveDashboard;

    vm.items = {};
    vm.enabledItems = {};
    vm.group = mnStoreService.store("groups").get($scope.groupID);

    maybeShowItemsControls();

    $scope.$watch("mnStatsGroupsCtl.items.index", onItemChange);
    $scope.$watch("mnStatsGroupsCtl.items.xdcr", onItemChange);
    $scope.$watch("mnStatsGroupsCtl.items.fts", onItemChange);
    $scope.$watch("mnStatsGroupsCtl.items.kv", onItemChange);

    function onItemChange() {
      vm.reloadChartDirective = true;
      $timeout(function () {
        vm.reloadChartDirective = false;
      });
    }

    function maybeShowItemsControls() {
      var items = {};
      ((vm.group || {}).charts || []).forEach(function (chartID) {
        var stats = mnStoreService.store("charts").get(chartID) ?
            mnStoreService.store("charts").get(chartID).stats : {};
        var chartStats = Object.keys(stats);
        chartStats.forEach(function (statPath) {
          if (statPath.includes("@items")) {
            items[statPath.split(".")[0]] = true;
          }
        });
      });
      vm.enabledItems = items;
    }

    function deleteGroup(groupID) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_group_delete.html',
      }).result.then(function () {
        mnStatisticsNewService.deleteGroup(groupID);
        mnUserRolesService.saveDashboard();
      });
    }

    function onGroupDelete() {
      vm.onControlClick = true;
      deleteGroup($scope.groupID);
      hideGroupControls();
    }

    function onGroupSubmit() {
      vm.initName = vm.group.name;
      mnUserRolesService.saveDashboard()
      hideGroupControls();
      vm.focusOnSubmit = true;
    }

    function onGroupFocus() {
      vm.showGroupControls = true;
      vm.initName = vm.group.name;
    }

    function onGroupNameBlur() {
      if (!vm.onControlClick) {
        vm.showGroupControls = false;
        vm.group.name = vm.initName;
        mnStoreService.store("groups").put(vm.group);
      }
    }

    function hideGroupControls() {
      if (vm.onControlClick) {
        vm.onControlClick = false;
        onGroupNameBlur();
      }
    }
  }

  function mnStatisticsNewController($scope, mnStatisticsNewService, $state, $http, mnPoller, mnBucketsService, $uibModal, $rootScope, mnHelper, $window, mnUserRolesService, permissions, $timeout,mnStoreService, mnGsiService, mnViewsListService, mnTasksDetails, $anchorScroll, $location) {
    var vm = this;

    vm.mnStatisticsNewScope = $scope;

    vm.onSelectScenario = onSelectScenario;
    vm.onSelectZoom = onSelectZoom;

    vm.bucket = $state.params.scenarioBucket;
    vm.zoom = $state.params.scenarioZoom;
    vm.node = $state.params.statsHostname;
    //selected scenario holder
    vm.scenario = {};
    vm.openGroupDialog = openGroupDialog;
    vm.selectedBucket = $state.params.scenarioBucket;
    vm.onBucketChange = onBucketChange;
    vm.onSelectNode = onSelectNode;

    vm.openChartBuilderDialog = openChartBuilderDialog;
    vm.resetDashboardConfiguration = resetDashboardConfiguration;
    vm.showBlocks = {
      "Server Resources": true
    };

    activate();

    function resetDashboardConfiguration() {
      return $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_reset_dialog.html'
      }).result.then(function () {
        mnStoreService.store("charts").clear();
        mnStoreService.store("groups").clear();
        mnStoreService.store("scenarios").clear();
        mnStatisticsNewService.doAddPresetScenario();

        vm.scenario.selected =
          mnStoreService.store("scenarios").last();

        $state.go("^.statistics", {
          scenario: mnStoreService.store("scenarios").last().id
        });

        return mnUserRolesService.saveDashboard();
      });
    }

    function openGroupDialog(scenario) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_group.html',
        controller: 'mnGroupDialogController as groupDialogCtl',
        resolve: {
          scenario: mnHelper.wrapInFunction(scenario)
        }
      }).result.then(function (group) {
        $location.hash('group-' + group.id);
        $anchorScroll();
      });
    }


    function openChartBuilderDialog(group, scenario, groupCtl) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_statistics_chart_builder.html',
        controller: 'mnStatisticsNewChartBuilderController as builderCtl',
        resolve: {
          scenario: mnHelper.wrapInFunction(scenario),
          chart: mnHelper.wrapInFunction(),
          group: mnHelper.wrapInFunction(group)
        }
      }).result.then(function () {
        mnUserRolesService.saveDashboard();
        groupCtl.maybeShowItemsControls();
      });
    }

    function onSelectNode(selectedHostname) {
      $state.go('^.statistics', {
        statsHostname: selectedHostname.indexOf("All Server Nodes") > -1 ? "all" :selectedHostname
      });
    }

    function onBucketChange(bucket) {
      $state.go('^.statistics', {
        scenarioBucket: bucket
      });
    }

    function onSelectScenario(scenario) {
      $state.go('^.statistics', {
        scenario: scenario.id,
      });
    }

    function onSelectZoom() {
      $state.go('^.statistics', {
        scenarioZoom: vm.zoom
      });
    }

    function initItemsDropdownSelect() {
      if ($scope.rbac.cluster.tasks.read) {
        new mnPoller($scope, function () {
          return mnTasksDetails.get().then(function (rv) {
            if (!$state.params.scenarioBucket) {
              return;
            }
            return rv.tasksXDCR.filter(function (row) {
              return row.source == $state.params.scenarioBucket;
            });
          });
        })
          .setInterval(10000)
          .subscribe("xdcrItems", vm)
          .reloadOnScopeEvent("reloadXdcrPoller")
          .cycle();
      }

      if ($scope.rbac.cluster.settings.fts.read) {
        new mnPoller($scope, function () {
          return $http.get('/_p/fts/api/index').then(function(rv) {
            return Object.keys(rv.data.indexDefs.indexDefs).reduce(function (acc, key) {
              var index = rv.data.indexDefs.indexDefs[key];
              if (index.sourceName == $state.params.scenarioBucket) {
                acc.push(index);
              }
              return acc;
            }, []);
          });
        })
          .setInterval(10000)
          .subscribe("ftsItems", vm)
          .reloadOnScopeEvent("reloadXdcrPoller")
          .cycle();
      }

      if ($scope.rbac.cluster.bucket['.'].n1ql.index.read) {
        new mnPoller($scope, function () {
          return mnGsiService.getIndexesState().then(function (rv) {
            if (!$state.params.scenarioBucket) {
              return;
            }
            return rv.byBucket[$state.params.scenarioBucket];
          });
        })
          .setInterval(10000)
          .subscribe("indexItems", vm)
          .reloadOnScopeEvent("indexStatusURIChanged")
          .cycle();
      }

      if ($scope.rbac.cluster.bucket['.'].views.read) {
        new mnPoller($scope, function () {
          return mnStatisticsNewService.getStatsDirectory($state.params.scenarioBucket, {})
            .then(function (rv) {
              if (!$state.params.scenarioBucket) {
                return;
              }
              return rv.data.blocks.filter(function (block) {
                if (block.blockName.includes("View Stats")) {
                  block.statId = block.blockName.split(": ")[1];
                  var name = block.stats[0].name.split("/");
                  name.pop()
                  block.statKeyPrefix = name.join("/") + "/";
                  return true;
                }
              });
            });
        })
          .setInterval(10000)
          .subscribe("viewItems", vm)
          .reloadOnScopeEvent("reloadViewsPoller")
          .cycle();
      }
    }

    function activate() {
      initItemsDropdownSelect();

      mnStatisticsNewService.heartbeat.setInterval(
        mnStatisticsNewService.defaultZoomInterval(vm.zoom));

      if ($scope.rbac.cluster.stats.read) {
        mnUserRolesService.getUserProfile().then(function (profile) {
          vm.scenario.selected =
            $state.params.scenario ?
            mnStoreService.store("scenarios").get($state.params.scenario) :
            mnStoreService.store("scenarios").last();
          vm.scenarios = mnStoreService.store("scenarios").share();
        });
      }

      new mnPoller($scope, function () {
        return mnStatisticsNewService.prepareNodesList($state.params);
      })
        .subscribe("nodes", vm)
        .reloadOnScopeEvent("nodesChanged")
        .cycle();
    }
  }
})();
