(function () {
  "use strict";

  angular
    .module('mnAuth')
    .controller('mnAuthController', mnAuthController);

  function mnAuthController(mnAuthService, $location, $state, $urlRouter) {
    var vm = this;

    vm.loginFailed = false;
    vm.submit = submit;

    activate();

    function activate() {
      if ($state.transition.$from().includes["app.wizard"]) {
        error({status: "initialized"})
      }

      mnAuthService.canUseCertForAuth().then(function (data) {
        vm.canUseCert = data.cert_for_auth;
      });
    }

    function error(resp) {
      vm.error = {};
      vm.error["_" + resp.status] = true;
    }
    function success() {
      /* never sync to /auth URL (as user will stay on the login page) */
      if ($location.path() === "/auth") {
        $state.go('app.admin.overview.statistics');
      } else {
        $urlRouter.sync();
      }
    }
    function submit(useCertForAuth) {
      mnAuthService
        .login(vm.user, useCertForAuth)
        .then(success, error);
    }
  }
})();
