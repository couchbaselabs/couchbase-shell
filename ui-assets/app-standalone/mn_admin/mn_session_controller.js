(function () {
  "use strict";

  angular.module('mnSession', [
    'mnSessionService',
    'mnPromiseHelper'
  ]).controller('mnSessionController', mnSessionController);

  function mnSessionController(mnSessionService, mnPromiseHelper) {
    var vm = this;

    vm.submit = submit;

    activate();

    function activate() {
      mnPromiseHelper(vm, mnSessionService.get())
        .applyToScope("state");
    }

    function submit() {
      if (vm.viewLoading) {
        return;
      }
      mnPromiseHelper(vm, mnSessionService.post(vm.state.uiSessionTimeout))
        .catchErrors()
        .showSpinner()
        .showGlobalSuccess("Session settings changed successfully!");
    };
  }
})();
