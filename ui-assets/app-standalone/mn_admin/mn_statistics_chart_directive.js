(function () {
  "use strict"

  angular
    .module('mnStatisticsChart', [
      "mnStatisticsNewService",
      "mnStatisticsDescriptionService",
      "ui.bootstrap",
      "mnPoll",
      "mnFilters",
      "mnHelper",
      "mnD3Service"
    ])
    .directive("mnStatisticsChart", mnStatisticsNewChartDirective);

  function mnStatisticsNewChartDirective(mnStatisticsNewService, mnStatisticsDescriptionService, $uibModal, $state, mnPrepareQuantityFilter, mnTruncateTo3DigitsFilter, $rootScope, mnHelper, $timeout, $window) {
    return {
      restrict: 'AE',
      templateUrl: 'app/mn_admin/mn_statistics_chart_directive.html',
      scope: {
        syncScope: "=?",
        config: "=",
        mnD3: "=?",
        bucket: "@",
        zoom: "@",
        node: "@?",
        items: "=?",
        api: "=?"
      },
      controller: controller
    };

    function controller($scope, $element) {
      if (!$scope.config) {
        return;
      }

      var units;
      var options;
      var isFocusChart = $scope.nvd3Options && $scope.nvd3Options.type === 'lineWithFocusChart';

      if (!_.isEmpty($scope.config.stats)) {
        units = mnStatisticsNewService.getStatsUnits($scope.config.stats);
        $scope.title = mnStatisticsNewService.getStatsTitle($scope.config.stats);
        $scope.desc = mnStatisticsNewService.getStatsDesc($scope.config.stats);
        activate();
      }

      function activate() {
        initConfig();
        subscribeToMultiChartData();
      }

      function subscribeToMultiChartData() {
        mnStatisticsNewService.subscribeUIStatsPoller({
          bucket: $scope.bucket,
          node: $scope.node || "all",
          stats: mnStatisticsNewService.descriptionPathsToStatNames($scope.config, $scope.items),
          zoom: $scope.zoom,
          specificStat: $scope.config.specificStat
        }, $scope);


        $scope.$watch("mnUIStats", onMultiChartDataUpdate);
      }

      function getChartSize(size) {
        switch (size) {
        case "tiny": return 60;
        case "small": return 100;
        case "medium": return 120;
        case "large": return 310;
        case "extra": return 430;
        default: return 150;
        }
      }

      function initConfig() {
        options = {
          chart: {
            margin : $scope.config.margin || {top: 10, right: 36, bottom: 16, left: 44},
            height: getChartSize($scope.config.size),
            tooltip: {valueFormatter: formatValue},
            useInteractiveGuideline: true,
            yAxis: [],
            xAxis: {
              tickFormat: function (d) {
                return mnStatisticsNewService.tickMultiFormat(new Date(d));
              }
            },
            noData: "Stats are not found or not ready yet"
          }
        };

        Object.keys(units).forEach(function (unit, index) {
          units[unit] = index;
          options.chart.yAxis[index] = {};
          options.chart.yAxis[index].unit = unit;
          options.chart.yAxis[index].tickFormat = function (d) {
            return formatMaxMin(d, unit);
          };
          options.chart.yAxis[index].domain = getScaledMinMax;
        });

        if ($scope.mnD3) {
          Object.assign(options.chart, $scope.mnD3);
        }

        $scope.options = options;
      }

      function formatMaxMin(d, unit) {
        switch (unit) {
        case "bytes":
          var val = mnPrepareQuantityFilter(d, 1024);
          return d3.format(".2s")(d/val[0]) + val[1];
        case "percent":
          return d3.format(".0%")(d / 100);
        case "second":
          return d3.format(".2s")(d) + 's';
        case "millisecond":
          return d3.format(".2s")(d / 1000) + 's';
        case "microsecond":
          return d3.format(".2s")(d / 1000000) + 's';
        case "nanoseconds":
          return d3.format(".2s")(d / 1000000000) + 's';
        default:
          return d3.format(".2s")(d);
        }
      }

      function formatValue(d, unit) {
        switch (unit) {
        case "bytes":
          var val = mnPrepareQuantityFilter(d, 1024);
          return [mnTruncateTo3DigitsFilter(d/val[0]), val[1]].join('');
        case "percent":
          return  mnTruncateTo3DigitsFilter(d) + "%";
        case "millisecond":
          return d3.format(".2s")(d / 1000) + 's';
        case "microsecond":
          return d3.format(".2s")(d / 1000000) + 's';
        case "nanoseconds":
          return d3.format(".2s")(d / 1000000000) + 's';
        default: return mnTruncateTo3DigitsFilter(d);
        }
      }

      function getScaledMinMax(chartData) {
        var min = d3.min(chartData, function (line) {return line.min/1.005;});
        var max = d3.max(chartData, function (line) {return line.max;});
        if (chartData[0] && chartData[0].unit == "bytes") {
          return [min <= 0 ? 0 : roundDownBytes(min), max == 0 ? 1 : roundUpBytes(max)];
        } else {
          return [min <= 0 ? 0 : roundDown(min), max == 0 ? 1 : roundUp(max)];
        }
      }

      // make 2nd digit either 0 or 5
      function roundUp(num) {
        var mag = Math.pow(10,Math.floor(Math.log10(num)));
        return(mag*Math.ceil(2*num/mag)/2);
      }

      function roundDown(num) {
        var mag = Math.pow(10,Math.floor(Math.log10(num)));
        return(mag*Math.floor(2*num/mag)/2);
      }

      function roundUpBytes(num) { // round up 3rd digit to 0
        var mag = Math.trunc(Math.log2(num)/10);
        var base_num = num/Math.pow(2,mag*10); // how many KB, MB, GB, TB, whatever
        var mag10 = Math.pow(10,Math.floor(Math.log10(base_num))-1);
        return Math.ceil(base_num/mag10) * mag10 * Math.pow(2,mag*10);
      }

      function roundDownBytes(num) {
        var mag = Math.trunc(Math.log2(num)/10);
        var base_num = num/Math.pow(2,mag*10);
        var mag10 = Math.pow(10,Math.floor(Math.log10(base_num))-1);
        return Math.floor(base_num/mag10) * mag10 * Math.pow(2,mag*10);
      }

      function onMultiChartDataUpdate(stats) {
        if (!stats) {
          return;
        }

        if (stats.status == 404) {
          $scope.options = {
            chart: {
              notFound: true,
              height: getChartSize($scope.config.size),
              margin : {top: 0, right: 0, bottom: 0, left: 0},
              type: 'multiChart',
              noData: "Stats are not found or not ready yet"
            }
          };
          $scope.chartData = [];
          return;
        }

        var chartData = [];
        if ($scope.config.specificStat) {
          var descPath = Object.keys($scope.config.stats)[0];
          var statName = Object.keys(stats.stats)[0];
          var desc = mnStatisticsNewService.readByPath(descPath);
          if (desc ) {
            (($scope.node == "all") ?
             Object.keys(stats.stats[statName] || {}) : ([$scope.node]))
              .forEach(function (nodeName) {
                var nodeStat = stats.stats[statName][nodeName];
                nodeStat = nodeStat.map(function (v) {
                  return (v === null) ? undefined : v;
                });
                chartData.push({
                  type: 'line',
                  unit: desc.unit,
                  max: d3.max(nodeStat || []) || 1,
                  min: d3.min(nodeStat || []) || 0,
                  yAxis: units[desc.unit],
                  key: nodeName,
                  values: stats.timestamps.map(function (v, i) {
                    return [v, nodeStat[i]];
                  })
                });
              });
          }
        } else {
          Object.keys($scope.config.stats).forEach(function (descPath, i) {
            var desc = mnStatisticsNewService.readByPath(descPath);
            if (!desc) {
              return;
            }
            var statName = mnStatisticsNewService
                .descriptionPathToStatName(descPath, $scope.items);
            var stat;
            if (stats.stats[statName]) {
              stat = stats.stats[statName][$scope.node == "all" ? "aggregate" : $scope.node];
            } else {
              stat = [];
            }
            stat = stat.map(function (v) {
              return (v === null) ? undefined : v;
            });
            chartData.push({
              type: 'line',
              unit: desc.unit,
              max: d3.max(stat) || 1,
              min: d3.min(stat) || 0,
              yAxis: units[desc.unit],
              key: desc.title,
              values: stats.timestamps.map(function (v, i) {
                return [v, stat[i]];
              })
            });
          });
        }
        if ($scope.chartData) {
          $scope.chartData.forEach(function (v, i) {
            if (!chartData[i]) {
              return;
            }
            chartData[i].disabled = v.disabled;
          });
        }

        $scope.chartData = chartData;
      }
    }
  }
})();
