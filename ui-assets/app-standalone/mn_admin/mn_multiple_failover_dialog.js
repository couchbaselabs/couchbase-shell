(function () {
  "use strict";

  angular
    .module('mnServers')
    .controller('mnMultipleFailoverDialogController', mnMultipleFailoverDialogController);

  function mnMultipleFailoverDialogController($scope, mnPoolDefault, mnServersService, mnPromiseHelper, groups, nodes, $uibModalInstance, $uibModal, mnHelper) {
    var vm = this;

    vm.nodes = nodes;
    vm.onSubmit = onSubmit;
    vm.mnGroups = groups;
    vm.mnSelectedNodesHolder = {};

    function doPostFailover(allowUnsafe) {
      var otpNodes = mnHelper.checkboxesToList(vm.mnSelectedNodesHolder);
      var promise = mnServersService.postFailover("startFailover", otpNodes, allowUnsafe);
      return mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors()
        .closeOnSuccess()
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
  }
})();
