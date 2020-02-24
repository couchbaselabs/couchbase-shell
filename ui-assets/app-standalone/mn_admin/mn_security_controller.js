(function () {
  "use strict";

  angular
    .module("mnSecurity")
    .controller("mnSecurityController", mnSecurityController);

  function mnSecurityController($scope, mnPluggableUiRegistry, poolDefault) {
    var vm = this;
    vm.poolDefault = poolDefault;
  }
})();
