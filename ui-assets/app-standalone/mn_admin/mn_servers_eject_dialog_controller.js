(function () {
  "use strict";

  angular
    .module('mnServers')
    .controller('mnServersEjectDialogController', mnServersEjectDialogController);

    function mnServersEjectDialogController($scope, $rootScope, $uibModalInstance, node, warnings, mnHelper, mnPromiseHelper, mnServersService) {
      var vm = this;
      vm.warningFlags = warnings;
      vm.doEjectServer = doEjectServer;

      function doEjectServer() {
        mnServersService.addToPendingEject(node);
        $uibModalInstance.close();
        $rootScope.$broadcast("reloadNodes");
      };
    }
})();
