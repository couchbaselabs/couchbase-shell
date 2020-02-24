(function () {
  "use strict";

  angular
    .module("mnUserRoles")
    .controller("mnUserRolesDeleteDialogController", mnUserRolesDeleteDialogController);

  function mnUserRolesDeleteDialogController($scope, mnUserRolesService, user, mnPromiseHelper, $uibModalInstance) {
    var vm = this;
    vm.username = user.id;
    vm.onSubmit = onSubmit;

    function onSubmit() {
      mnPromiseHelper(vm, mnUserRolesService.deleteUser(user), $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .broadcast("reloadRolesPoller")
        .showGlobalSuccess("User deleted successfully!");
    }
  }
})();
