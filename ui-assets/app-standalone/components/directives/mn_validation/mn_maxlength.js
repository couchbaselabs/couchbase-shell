(function () {
  "use strict";

  angular
    .module('mnMaxlength', [])
    .directive('mnMaxlength', mnMaxlengthDirective);

  function mnMaxlengthDirective() {
    var mnMaxlength = {
      restrict: 'A',
      require: 'ngModel',
      link: link
    };
    return mnMaxlength;

    function link(scope, element, attrs, ctrl) {

      ctrl.$parsers.unshift(function (value) {
        var max = attrs.mnMaxlength;
        ctrl.$setValidity('mnMaxlength', max && value && value.length <= parseInt(max));
        return value;
      });
    }
  }
})();
