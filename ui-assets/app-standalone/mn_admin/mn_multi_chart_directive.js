(function () {
  "use strict"

  angular
    .module('mnStatisticsChart')
    .directive("mnMultiChart", mnMultiChartDirective);

  function mnMultiChartDirective($window, mnD3Service) {
    return {
      restrict: 'AE',
      scope: {
        data: "=?",
        options: "=?",
        api: "=?",
        syncScope: "=?"
      },
      controller: controller
    };

    function controller($element, $scope) {
      var chart = new mnD3Service.mnD3Tooltip($scope.options, $element, function () {
        angular.element($window).on('resize', chart.throttledResize);
        chart.resize();
        if ($scope.syncScope) {
          syncTooltips();
        }
        if ($scope.options.chart.showFocus) {
          addFocusChart();
        }
        if (this.cht.showLegends) {
          this.drawLegends();
        }
      });

      $scope.$watch("data", chart.updateData.bind(chart));
      $scope.$on("$destroy", function () {
        angular.element($window).off('resize', chart.throttledResize);
        chart.destroy.bind(chart);
      });

      if ($scope.api) {
        $scope.api.chart = chart;
      }

      function addFocusChart() {
        var focusChartOpt = _.cloneDeep($scope.options);
        focusChartOpt.chart.height = 80;
        focusChartOpt.chart.hideTicks = true;
        focusChartOpt.chart.showFocus = false;
        var chartF = new mnD3Service.mnD3Focus(focusChartOpt, $element, chart);
        angular.element($window).on('resize', chartF.throttledResize);

        $scope.$watch("data", chartF.updateData.bind(chartF));

        chart.rootEl.on("toggleLegend", function (opt) {
          chartF.updateData(chartF.data);
        });

        if ($scope.api) {
          $scope.api.chartF = chartF;
        }

        $scope.$on("$destroy", function () {
          angular.element($window).off('resize', chartF.throttledResize);
          chartF.destroy.bind(chartF);
        });
      }

      function syncTooltips() {
        var throttledSync = _.throttle(function (e) {
          if (e.bubbles) {
            $scope.syncScope.$broadcast("syncTooltips", {
              element: $element,
              event: e,
              chart: chart
            });
          }
        }, 10, {leading: true});

        angular.element(chart.tipBox.node()).on("mousemove mouseup mousedown mouseout",
                                                throttledSync);

        $scope.$on("$destroy", function () {
          angular.element(chart.tipBox.node()).off("mousemove mouseup mousedown mouseout",
                                                   throttledSync);
        });

        $scope.$on("syncTooltips", function (e, source) {
          if (source.element[0] !== $element[0]) {
            var sourcePos = source.chart.tipBox.node().getBoundingClientRect();
            var elementPos = chart.tipBox.node().getBoundingClientRect();
            var sourceGraphRelativeX = source.event.clientX - sourcePos.x;
            var sourceGraphRelativeY = source.event.clientY - sourcePos.y;

            var interpolateX = sourcePos.width / sourceGraphRelativeX;
            var clientX = elementPos.x + (elementPos.width / interpolateX);

            var interpolateY = sourcePos.height / sourceGraphRelativeY;
            var clientY = elementPos.y + (elementPos.height / interpolateY);

            source.chart.disableTooltip(false);
            chart.disableTooltip(true);

            chart.tipBox.node().dispatchEvent(createEvent(
	      source.event.type,
              clientX,
              clientY
	    ));
          }
        });
      }

      function createEvent(type, clientX, clientY){
        var event = new MouseEvent(type, {
          view: $window,
          bubbles: false,
          cancelable: true,
          clientX: clientX,
          clientY: clientY
        });
        return event;
      }
    }
  }

})();
