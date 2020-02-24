(function () {
  "use strict";

  angular
    .module('mnServers')
    .controller('mnServersFailOverDialogController', mnServersFailOverDialogController);

  function mnServersFailOverDialogController($scope, mnServersService, mnPromiseHelper, node, $uibModalInstance, $uibModal) {
    var vm = this;

    vm.node = node;
    vm.onSubmit = onSubmit;
    vm.isFailOverBtnDisabled = isFailOverBtnDisabled;

    activate();

    function isFailOverBtnDisabled() {
      return !vm.status || !vm.status.confirmation &&
             (vm.status.failOver === 'startFailover') &&
            !(vm.status.down && !vm.status.backfill) && !vm.status.dataless;
    }

    function doPostFailover(allowUnsafe) {
      var promise = mnServersService.postFailover(vm.status.failOver, node.otpNode, allowUnsafe);
      return mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .broadcast("reloadServersPoller");
    }

    function onSubmit() {
      doPostFailover()
        .getPromise()
        .then(null, function (resp) {
          if (resp.status == 504) {
            return $uibModal.open({
              templateUrl: 'app/mn_admin/mn_servers_failover_confirmation_dialog.html'
            }).result.then(function () {
              return doPostFailover(true);
            });
          }
        });
    }
    function activate() {
      mnPromiseHelper(vm, mnServersService.getNodeStatuses(node.hostname))
        .showSpinner()
        .getPromise()
        .then(function (details) {
          if (details) {
            vm.status = details;
          } else {
            $uibModalInstance.close();
          }
        });
    }
  }
})();
