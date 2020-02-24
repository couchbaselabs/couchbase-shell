(function () {
  "use strict";

  angular
    .module("mnPoorMansAlerts", [
      "mnPromiseHelper",
      "mnPoorMansAlertsService",
      "mnSpinner"
    ])
    .controller("mnPoorMansAlertsController", mnPoorMansAlertsController);

  function mnPoorMansAlertsController(mnPromiseHelper, mnPoorMansAlertsService, alertsSilenceURL, alerts, $uibModalInstance) {
    var vm = this;

    vm.alerts = alerts;
    vm.onClose = onClose;

    function onClose() {
      mnPromiseHelper(vm, mnPoorMansAlertsService.postAlertsSilenceURL(alertsSilenceURL), $uibModalInstance)
        .showGlobalSpinner()
        .closeOnSuccess();
    }
  }
})();
