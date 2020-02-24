(function () {
  "use strict";

  angular
    .module('mnSpinner', [])
    .directive('mnSpinner', mnSpinnerDirective);

    function mnSpinnerDirective($compile) {
      var directive = {
        restrict: 'A',
        scope: {
          mnSpinner: '=',
          minHeight: '@',
          opacity: '@'
        },
        link: link
      };

      return directive;

      function link($scope, $element) {
        var spinner = angular.element("<div class=\"spinner\" ng-show=\"mnSpinner\"></div>");
        if ($scope.opacity) {
          spinner.addClass("opacity");
        }
        if ($scope.minHeight) {
          spinner.css({minHeight: $scope.minHeight});
        }
        $element.addClass("relative");
        $element.append($compile(spinner)($scope));
      }
    }
})();
