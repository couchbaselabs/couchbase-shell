(function () {
  "use strict";

  angular
    .module('mnStatisticsNew')
    .controller('mnGroupDialogController', mnGroupDialogController)

  function mnGroupDialogController($uibModalInstance, mnUserRolesService, mnPromiseHelper, scenario, mnStoreService) {
    var vm = this;
    vm.group = {
      name: "",
      desc: "",
      charts: [],
      isOpen: true
    };

    vm.submit = submit;

    function submit() {
      var group = mnStoreService.store("groups").add(vm.group);
      scenario.groups.push(group.id);

      mnPromiseHelper(vm, mnUserRolesService.saveDashboard())
        .showGlobalSpinner()
        .showGlobalSuccess("Group added successfully!")
        .onSuccess(function () {
          $uibModalInstance.close(group);
        });
    }
  }

})();
