(function () {
  "use strict";

  angular
    .module('mnBuckets')
    .controller('mnBucketsDetailsDialogController', mnBucketsDetailsDialogController);

  function mnBucketsDetailsDialogController($scope, $rootScope, $state, mnBucketsDetailsDialogService, bucketConf, autoCompactionSettings, mnHelper, mnPromiseHelper, $uibModalInstance, mnAlertsService) {
    var vm = this;
    if (autoCompactionSettings !== undefined) {
      bucketConf.autoCompactionDefined = !!bucketConf.autoCompactionSettings;
      vm.autoCompactionSettings = autoCompactionSettings;
    }
    vm.bucketConf = bucketConf;
    vm.validationKeeper = {};
    vm.onSubmit = onSubmit;
    vm.$uibModalInstance = $uibModalInstance;

    function onSubmit() {
      var data = mnBucketsDetailsDialogService.prepareBucketConfigForSaving(vm.bucketConf, vm.autoCompactionSettings, $scope.poolDefault, $scope.pools);
      var promise = mnBucketsDetailsDialogService.postBuckets(data, vm.bucketConf.uri);

      mnPromiseHelper(vm, promise)
        .showGlobalSpinner()
        .catchErrors(function (result) {
          if (result) {
            if (result.summaries) {
              vm.validationResult = mnBucketsDetailsDialogService.adaptValidationResult(result);
            } else {
              mnAlertsService.showAlertInPopup(result, "error");
            }
          }
        })
        .onSuccess(function (result) {
          if (!result.data) {
            $uibModalInstance.close();
            $rootScope.$broadcast("reloadBucketStats");
          }
        })
        .showGlobalSuccess("Bucket settings saved successfully!");
    };
  }
})();
