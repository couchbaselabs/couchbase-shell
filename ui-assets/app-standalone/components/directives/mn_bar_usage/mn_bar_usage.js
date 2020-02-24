(function () {
  "use strict";

  angular
    .module('mnBarUsage', [
      'mnFilters'
    ])
    .directive('mnBarUsage', mnBarUsageDirective);

  function mnBarUsageDirective(mnRescaleForSumFilter, mnCalculatePercentFilter) {

    var mnBarUsage = {
      restrict: 'A',
      scope: {
        baseInfo: '=',
      },
      isolate: false,
      templateUrl: 'app/components/directives/mn_bar_usage/mn_bar_usage.html',
      controller: controller
    };

    return mnBarUsage;

    function controller($scope) {
      $scope.$watch('baseInfo', function (options) {
        if (!options) {
          return;
        }
        var sum = 0;
        var newOptions = _.cloneDeep(options);
        var items = newOptions.items;
        var values = _.map(items, function (item) {
          return Math.max(item.value, 0);
        });
        var total = _.chain(values).reduce(function (sum, num) {
          return sum + num;
        }).value();

        values = mnRescaleForSumFilter(100, values, total);

        _.each(values, function (item, i) {
          var v = values[i];
          values[i] += sum;
          newOptions.items[i].itemStyle = newOptions.items[i].itemStyle || {};
          newOptions.items[i].itemStyle.width = values[i] + "%";
          sum += v;
        });
        newOptions.tdItems = _.select(newOptions.items, function (item) {
          return item.name !== null;
        });

        $scope.config = newOptions;
      }, true);
    }
  }
})();
