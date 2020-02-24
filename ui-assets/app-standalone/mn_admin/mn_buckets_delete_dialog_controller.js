(function () {
  "use strict";

  angular
    .module('mnBuckets')
    .controller('mnBucketsDeleteDialogController', mnBucketsDeleteDialogController);

  function mnBucketsDeleteDialogController($scope, $uibModalInstance, bucket, mnHelper, mnPromiseHelper, mnBucketsDetailsService, mnAlertsService) {
    var vm = this;
    vm.doDelete = doDelete;
    vm.bucketName = bucket.name;

    function doDelete() {
      var promise = mnBucketsDetailsService.deleteBucket(bucket);
      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .catchGlobalErrors()
        .closeFinally()
        .showGlobalSuccess("Bucket deleted successfully!");
    }
  }
})();
