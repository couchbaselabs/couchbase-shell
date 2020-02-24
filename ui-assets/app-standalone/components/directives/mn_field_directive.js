(function () {
  "use strict";

  angular
    .module('mnField', [])
    .directive('mnField', mnFieldDirective);

  function mnFieldDirective() {
    var mnFieldDirective = {
      restrict: "AE",
      scope: {
        mnName: "@",
        mnType: "@",
        mnId: "@",
        mnDisabled: "=",
        mnLabel: "@",
        mnErrors: "=?",
        mnModel: "=",
        mnItems: "="
      },
      templateUrl: "app/components/directives/mn_field.html",
      controller: controller,
      controllerAs: "thisCtl"
    };

    return mnFieldDirective;

    function controller($scope) {

    }
  }
})();
