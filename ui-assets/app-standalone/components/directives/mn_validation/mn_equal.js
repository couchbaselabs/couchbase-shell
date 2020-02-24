(function () {
  "use strict";

  angular
    .module('mnEqual', [])
    .directive('mnEqual', mnEqualDirective);

  function mnEqualDirective() {
    var mnEqual = {
      restrict: 'A',
      require: 'ngModel',
      link: link
    };
    return mnEqual;

    function link(scope, element, attrs, ctrl) {
      function validate(value) {
        ctrl.$setValidity('mnEqual', (value === undefined ? "" : value) === attrs.mnEqual);
        return value;
      };

      ctrl.$parsers.unshift(validate);
      ctrl.$formatters.push(validate);

      attrs.$observe('mnEqual', function () {
        return validate(ctrl.$viewValue);
      });
    }
  }
})();
