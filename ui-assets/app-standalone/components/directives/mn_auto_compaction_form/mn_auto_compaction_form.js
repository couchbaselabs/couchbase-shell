(function () {
  "use strict";

  angular
    .module('mnAutoCompactionForm', ['mnPeriod'])
    .directive('mnAutoCompactionForm', mnAutoCompactionFormDirective);

  function mnAutoCompactionFormDirective($http, daysOfWeek, mnPermissions, mnPoolDefault, mnPromiseHelper, mnSettingsClusterService) {
    var mnAutoCompactionForm = {
      restrict: 'A',
      scope: {
        autoCompactionSettings: '=',
        validationErrors: '=',
        isBucketsSettings: '='
      },
      isolate: false,
      replace: true,
      templateUrl: 'app/components/directives/mn_auto_compaction_form/mn_auto_compaction_form.html',
      controller: controller
    };

    function controller($scope) {
      $scope.daysOfWeek = daysOfWeek;
      $scope.rbac = mnPermissions.export;
      $scope.poolDefault = mnPoolDefault.export;
      $scope.maybeDisableTimeInterval = maybeDisableTimeInterval;
      $scope.props = {};

      if ($scope.rbac.cluster.settings.indexes.read) {
        mnPromiseHelper($scope, mnSettingsClusterService.getIndexSettings())
          .applyToScope(function (indexSettings) {
            $scope.indexSettings = indexSettings;
            maybeDisableTimeInterval();
          });
      }

      function isFragmentationProvided(value) {
        return (value.percentageFlag && value.percentage) ||
          (value.sizeFlag && value.size);
      }

      function maybeDisableTimeInterval() {
        $scope.props.isFragmentationProvided =
          isFragmentationProvided($scope.autoCompactionSettings.databaseFragmentationThreshold) ||
          isFragmentationProvided($scope.autoCompactionSettings.viewFragmentationThreshold);
        if (!$scope.props.isFragmentationProvided) {
          $scope.autoCompactionSettings.allowedTimePeriodFlag = false;
        }
      }
    }

    return mnAutoCompactionForm;
  }
})();
