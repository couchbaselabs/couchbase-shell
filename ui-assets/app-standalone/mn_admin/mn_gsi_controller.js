(function () {
  "use strict";

  angular.module('mnGsi', [
    'mnHelper',
    'mnGsiService',
    'mnSortableTable',
    'mnPoll',
    'mnPoolDefault',
    'mnPermissions',
    'mnSpinner',
    'mnFilters',
    'mnSearch',
    'mnElementCrane',
    'ui.bootstrap',
    'mnPromiseHelper',
    'mnAlertsService',
    'mnStatisticsNewService',
    'mnDetailStats'
  ]).controller('mnGsiController', mnGsiController)
    .controller('mnFooterStatsController', mnFooterStatsController);

  function mnGsiController($scope, mnGsiService, mnPoller) {
    var vm = this;
    activate();

    function activate() {
      new mnPoller($scope, function () {
        return mnGsiService.getIndexesState();
      })
        .setInterval(10000)
        .subscribe("state", vm)
        .reloadOnScopeEvent("indexStatusURIChanged")
        .cycle();
    }
  }

  function mnFooterStatsController($scope, mnStatisticsNewService, mnPermissions) {
    var vm = this;
    vm.currentBucket = mnPermissions.export.bucketNames['.stats!read'] &&
      mnPermissions.export.bucketNames['.stats!read'][0];
    vm.onSelectBucket = onSelectBucket;

    vm.getLatestStat = getLatestStat;
    vm.getLatestStatExponent = getLatestStatExponent;

    var config = {
      bucket: vm.currentBucket,
      node: "all",
      zoom: 3000,
      step: 1,
      stats: $scope.stats
    };

    activate();

    function activate() {
      mnStatisticsNewService.subscribeUIStatsPoller(config, $scope);
    }

    function getLatestStat(statName) {
      return $scope.mnUIStats &&
        $scope.mnUIStats.stats[statName] &&
        $scope.mnUIStats.stats[statName].aggregate.slice().reverse().find(stat => stat != null);
    }

    function getLatestStatExponent(statName, digits) {
      var value = getLatestStat(statName);
      if (value) {
        return(d3.format('.'+digits+'e')(value));
      } else {
        return value;
      }
    }

    function onSelectBucket() {
      config.bucket = vm.currentBucket;
      mnStatisticsNewService.heartbeat.throttledReload();
    }

  }
})();
