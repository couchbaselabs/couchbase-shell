(function () {
  "use strict";

  angular
    .module("mnRedaction", [
      "mnLogRedactionService",
      "mnSpinner"
    ])
    .controller("mnRedactionController", mnRedactionController);

  function mnRedactionController($scope, mnLogRedactionService, mnPromiseHelper) {
    var vm = this;
    vm.onSubmit = onSubmit;

    activate();

    function onSubmit() {
      if ($scope.mnGlobalSpinnerFlag) {
        return;
      }

      mnPromiseHelper(vm, mnLogRedactionService.post(vm.logRedactionSettings))
        .showGlobalSpinner()
        .catchErrors()
        .showGlobalSuccess("Settings saved successfully!");
    }

    function activate() {
      mnPromiseHelper(vm, mnLogRedactionService.get())
        .applyToScope("logRedactionSettings");
    }
  }
})();
