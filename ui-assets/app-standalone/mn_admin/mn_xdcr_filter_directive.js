(function () {
  "use strict";

  angular
    .module('mnXDCR')
    .directive('mnXdcrFilter', mnXdcrFilterDirective);

  function mnXdcrFilterDirective($http, mnPromiseHelper, mnXDCRService) {
      var mnXdcrFilter = {
        restrict: 'AE',
        scope: {
          mnReplication: '=',
          mnIsEditing: "=?",
          mnErrors: "=?"
        },
        templateUrl: 'app/mn_admin/mn_xdcr_filter.html',
        controller: controller,
        controllerAs: "xdcrFilterCtl",
      };

      return mnXdcrFilter;

      function controller($scope) {
        var vm = this;

        vm.onExpressionUpdate = onExpressionUpdate;
        vm.showAdvancedFiltering = $scope.mnIsEditing;

        function onExpressionUpdate() {
          handleValidateRegex($scope.mnReplication.filterExpression,
                              vm.testDocID, $scope.mnReplication.fromBucket);
        }

        function handleValidateRegex(regex, testDocID, bucket) {
          if (!testDocID) {
            return;
          }
          return mnPromiseHelper(vm, mnXDCRService.validateRegex(regex, testDocID, bucket))
            .showSpinner("filterExpressionSpinner")
            .catchErrors("filterExpressionError")
            .applyToScope("filterExpressionResult");
        }
      }
    }
})();
