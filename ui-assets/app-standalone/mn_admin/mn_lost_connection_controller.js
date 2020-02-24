(function () {
  "use strict";

  angular
    .module("mnLostConnection")
    .controller("mnLostConnectionController", mnLostConnectionController);

  function mnLostConnectionController($scope, mnLostConnectionService, $window, mnHelper) {
    var vm = this;
    vm.lostConnectionAt = $window.location.host;
    vm.state = mnLostConnectionService.getState();
    vm.retryNow = mnLostConnectionService.resendQueries;
  }
})();
