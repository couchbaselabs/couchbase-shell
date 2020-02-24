(function () {
  "use strict";

  angular
    .module('mnAnalytics')
    .controller('mnAnalyticsListGraphController', mnAnalyticsListGraphController);

  function mnAnalyticsListGraphController($scope, $rootScope, $transition$, $state) {
    var vm = this;
    var selectedStat;
    activate();

    function activate() {
      $rootScope.$broadcast('reloadAnalyticsPoller');
      $scope.$watch('analyticsCtl.state', watchOnAnalyticsState);
      $scope.$on('$destroy', onScopeDestroy);
      if ($state.params.specificStat) {
        $state.current.data.childParams.transGraph = $state.params.specificStat;
        $state.current.data.childParams.transZoom = $state.params.zoom;
      }
    }
    function onScopeDestroy() {
      selectedStat && (selectedStat.config.isSelected = false);
    }
    function watchOnAnalyticsState(analyticsState) {
      if (!analyticsState) {
        return;
      }

      selectedStat = analyticsState.statsByName && analyticsState.statsByName[$transition$.params().graph];
      if (!selectedStat) {
        vm.selectedStat = {};
        return;
      }

      selectedStat.column = $state.params.columnName;

      // notify plot of small graphs about selection
      selectedStat.config.isSelected = true;

      selectedStat.visiblePeriod = Math.ceil(Math.min(selectedStat.config.zoomMillis, analyticsState.stats.serverDate - selectedStat.config.timestamp[0]) / 1000);
      selectedStat.graphConfig = {
        stats: selectedStat.config.data,
        tstamps: selectedStat.config.timestamp,
        options: {
          color: '#1d88ad',
          verticalMargin: 1.02,
          fixedTimeWidth: selectedStat.config.zoomMillis,
          timeOffset: selectedStat.config.timeOffset,
          lastSampleTime: selectedStat.config.now,
          breakInterval: selectedStat.config.breakInterval,
          maxY: selectedStat.config.maxY,
          isBytes: selectedStat.config.isBytes
        }
      };
      analyticsState.selectedStat = selectedStat;
      vm.selectedStat = selectedStat;
    }
  }
})();
