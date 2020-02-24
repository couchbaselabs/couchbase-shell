(function () {
  "use strict";

  angular
    .module("mnResetPasswordDialog", [
      "mnResetPasswordDialogService",
      "mnAuthService",
      "mnFilters",
      "mnEqual"
    ])
    .controller("mnResetPasswordDialogController", mnResetPasswordDialogController);

  function mnResetPasswordDialogController($scope, mnResetPasswordDialogService, mnPromiseHelper, mnAuthService, user) {
    var vm = this;
    vm.submit = submit;
    vm.user = {
      name: user.id
    };

    function submit() {
      if (vm.form.$invalid) {
        return;
      }
      var promise = mnResetPasswordDialogService.post(vm.user);

      mnPromiseHelper(vm, promise)
        .showGlobalSpinner()
        .catchErrors()
        .onSuccess(function () {
          return mnAuthService.logout();
        });
    }
  }
})();
