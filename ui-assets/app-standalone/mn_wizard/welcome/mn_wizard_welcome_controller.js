(function () {
  "use strict";

  angular
    .module("mnWizard")
    .controller("mnWizardWelcomeController", mnWizardWelcomeController);

  function mnWizardWelcomeController(pools, mnWizardService) {
    var vm = this;

    vm.implementationVersion = pools.implementationVersion;
    vm.setIsNewClusterFlag = setIsNewClusterFlag;

    function setIsNewClusterFlag(value) {
      mnWizardService.getState().isNewCluster = value;
    }
  }
})();
