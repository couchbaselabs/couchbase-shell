(function () {
  "use strict";

  angular
    .module('mnGroups')
    .controller('mnGroupsGroupDialogController', mnGroupsGroupDialogController);

  function mnGroupsGroupDialogController($scope, $uibModalInstance, mnGroupsService, mnPromiseHelper, group) {
    var vm = this;

    vm.isEditMode = !!group;
    vm.groupName = group ? group.name : "";
    vm.onSubmit = onSubmit;

    function onSubmit() {
      if (vm.viewLoading) {
        return;
      }

      var promise = vm.isEditMode ? mnGroupsService.updateGroup(vm.groupName, group.uri) :
                                    mnGroupsService.createGroup(vm.groupName);
      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchErrors()
        .closeOnSuccess()
        .reloadState("app.admin.servers.list.groups")
        .showGlobalSuccess("Group saved successfully!");
    }
  }
})();
