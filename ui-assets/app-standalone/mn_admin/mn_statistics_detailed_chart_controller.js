(function () {
  "use strict";

  angular
    .module("mnStatisticsNew")
    .controller("mnStatisticsDetailedChartController", mnStatisticsDetailedChartController)

  function mnStatisticsDetailedChartController($scope, chart, $timeout, $state, items, mnStatisticsNewService) {
    var vm = this;
    vm.chart = Object.assign({}, chart, {size: "extra"});

    vm.items = items;
    vm.onSelectZoom = onSelectZoom;
    vm.bucket = $state.params.scenarioBucket;
    vm.zoom = $state.params.scenarioZoom !== "minute" ? $state.params.scenarioZoom : "hour";
    vm.node = $state.params.statsHostname;
    vm.options = {showFocus: true, showTicks: true, showLegends: true};

    mnStatisticsNewService.heartbeat.setInterval(
      mnStatisticsNewService.defaultZoomInterval(vm.zoom));

    function onSelectZoom() {
      vm.options.showFocus = vm.zoom !== "minute";
      mnStatisticsNewService.heartbeat.setInterval(
        mnStatisticsNewService.defaultZoomInterval(vm.zoom));
      vm.reloadChartDirective = true;
      $timeout(function () {
        vm.reloadChartDirective = false;
      });
    }

    $scope.$on("$destroy", function () {
      mnStatisticsNewService.heartbeat.setInterval(
        mnStatisticsNewService.defaultZoomInterval($state.params.scenarioZoom));
      mnStatisticsNewService.heartbeat.reload();
    })

  }
})();
