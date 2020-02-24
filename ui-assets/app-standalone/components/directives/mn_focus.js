(function () {
  "use strict";

  angular
    .module('mnFocus', [])
    .directive('mnFocus', mnFocusDirective);

  function mnFocusDirective($parse) {
    var mnFocus = {
      link: link
    };

    return mnFocus;

    function link($scope, $element, $attrs) {

      if ($attrs.mnFocus === "") {
        return $element[0].focus();
      }

      var getter = $parse($attrs.mnFocus);
      var setter = getter.assign;
      $scope.$watch($attrs.mnFocus, function (focus) {
        focus && $element[0].focus();
      });

      if (setter) {
        var handler = function handler() {
          setter($scope, false);
        }
        $element.on('blur', handler);
        $scope.$on('$destroy', function () {
          $element.off('blur', handler);
        })
      }
    }
  }
})();
