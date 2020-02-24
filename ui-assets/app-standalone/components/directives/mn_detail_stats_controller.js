(function () {
  "use strict";

  angular
    .module('mnDetailStats', ["mnStatisticsNewService", "mnStatisticsChart", "mnHelper", "mnStatisticsDescriptionService"])
    .directive('mnDetailStats', mnDetailStatsDirective);

  function mnDetailStatsDirective(mnStatisticsNewService, mnStatisticsDescriptionService, mnHelper) {
    var mnDetailStats = {
      restrict: "AE",
      scope: {
        mnTitle: "@",
        bucket: "@",
        itemId: "@",
        service: "@",
        prefix: "@",
        nodeName: "@?"
      },
      templateUrl: "app/components/directives/mn_detail_stats.html",
      controller: controller,
      controllerAs: "thisCtl"
    };

    return mnDetailStats;

    function controller($scope) {
      var vm = this;
      vm.zoom = "minute";
      vm.onSelectZoom = onSelectZoom;
      vm.items = {};

      activate();

      function onSelectZoom() {
        activate();
      }

      function getStats(stat) {
        var rv = {};
        rv["@" + $scope.service + "-.@items." + stat] = true;
        return rv;
      }

      function activate() {
        mnStatisticsNewService.heartbeat.setInterval(
          mnStatisticsNewService.defaultZoomInterval(vm.zoom));
        vm.items[$scope.service] = $scope.prefix + "/" + $scope.itemId + "/";
        vm.charts = Object
          .keys(mnStatisticsDescriptionService.stats["@" + $scope.service + "-"]["@items"])
          .filter(function (key) {
            return mnStatisticsDescriptionService.stats["@" +$scope.service+"-"]["@items"][key];
          })
          .map(function (stat) {
            return {
              node: $scope.nodeName,
              preset: true,
              id: mnHelper.generateID(),
              isSpecific: false,
              size: "small",
              stats: getStats(stat)
            };
          });
      }
    }
  }
})();
