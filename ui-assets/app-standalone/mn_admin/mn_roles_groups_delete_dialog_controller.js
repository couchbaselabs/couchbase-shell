(function () {
  "use strict";

  angular
    .module("mnRolesGroups")
    .controller("mnRolesGroupsDeleteDialogController", mnRolesGroupsDeleteDialogController);

  function mnRolesGroupsDeleteDialogController($scope, mnUserRolesService, rolesGroup, mnPromiseHelper, $uibModalInstance) {
    var vm = this;
    vm.grolesGroupsId = rolesGroup.id;
    vm.onSubmit = onSubmit;

    function onSubmit() {
      mnPromiseHelper(vm, mnUserRolesService.deleteRolesGroup(rolesGroup), $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .broadcast("reloadRolesGroupsPoller")
        .showGlobalSuccess("Group deleted successfully!");
    }
  }
})();
