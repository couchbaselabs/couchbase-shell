(function () {
  "use strict";

  angular
    .module('mnMinlength', [])
    .directive('mnMinlength', mnMinlengthDirective);

  function mnMinlengthDirective() {
    var mnMinlength = {
      restrict: 'A',
      require: 'ngModel',
      link: link
    };
    return mnMinlength;

    function link(scope, element, attrs, ctrl) {

      ctrl.$parsers.unshift(function (value) {
        var min = attrs.mnMinlength;
        ctrl.$setValidity('mnMinlength', min && value && value.length >= parseInt(min));
        return value;
      });
    }
  }
})();
