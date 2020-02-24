(function () {
  "use strict";

  angular
    .module("mnRootCertificate", [
      "mnRootCertificateService",
      "mnPromiseHelper",
      "mnSpinner"
    ])
    .controller("mnRootCertificateController", mnRootCertificateController);

  function mnRootCertificateController($scope, mnRootCertificateService, mnPromiseHelper) {
    var vm = this;

    activate();

    function activate() {
      mnPromiseHelper(vm, mnRootCertificateService.getDefaultCertificate())
        .applyToScope("certificate");
    }
  }
})();
