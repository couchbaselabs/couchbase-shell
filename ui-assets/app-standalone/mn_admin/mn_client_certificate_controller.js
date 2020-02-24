(function () {
  "use strict";

  angular
    .module("mnClientCertificate", [
      "mnClientCertificateService",
      "mnSpinner"
    ])
    .controller("mnClientCertController", mnClientCertController);


  function mnClientCertController($scope, mnClientCertificateService, mnPromiseHelper) {
    var vm = this;
    vm.onSubmit = onSubmit;

    activate();

    function maybeSetInititalValue(array, value) {
    }

    function onSubmit() {
      if ($scope.mnGlobalSpinnerFlag) {
        return;
      }

      mnPromiseHelper(vm, mnClientCertificateService.postClientCertificateSettings(vm.settings))
        .showGlobalSpinner()
        .catchErrors()
        .showGlobalSuccess("Settings saved successfully!");
    }

    function activate() {
      mnPromiseHelper(vm, mnClientCertificateService.getClientCertificateSettings())
        .applyToScope("settings")
        .onSuccess(function (resp) {
          if ($scope.rbac.cluster.admin.security.write && vm.settings.prefixes.length === 0) {
            vm.settings.prefixes.push({delimiter: '', prefix: '', path: 'subject.cn'});
          }
        });
    }
  }
})();
