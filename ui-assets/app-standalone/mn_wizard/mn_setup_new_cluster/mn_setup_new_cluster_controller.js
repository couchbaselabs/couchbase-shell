(function () {
  "use strict"

  angular
    .module('mnWizard')
    .controller('mnSetupNewClusterController', mnSetupNewClusterController);

  function mnSetupNewClusterController($state, mnClusterConfigurationService, mnPromiseHelper, mnWizardService) {
    var vm = this;

    vm.state = mnWizardService.getNewClusterState();
    vm.onSubmit = onSubmit;

    activate();

    function activate() {
      vm.focusMe = true;
    }
    function login(user) {
      return mnClusterConfigurationService.postAuth(user, true).then(function () {
        return $state.go('app.wizard.termsAndConditions');
      });
    }
    function onSubmit() {
      if (vm.viewLoading) {
        return;
      }

      if (vm.form.$invalid) {
        return activate();
      }

      var promise = login(vm.state.user);
      mnPromiseHelper(vm, promise)
        .showGlobalSpinner()
        .catchErrors(function (data) {
          vm.errors = (data && data.statusText) || data;
        });
    }
  }
})();
