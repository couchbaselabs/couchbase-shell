(function () {
  "use strict";

  angular
    .module('mnBuckets', [
      'mnHelper',
      'mnBucketsService',
      'ui.bootstrap',
      'mnBucketsDetailsDialogService',
      'mnBarUsage',
      'mnBucketsForm',
      'mnPromiseHelper',
      'mnPoll',
      'mnPoolDefault',
      'mnSpinner',
      'mnFilters',
      'mnTasksDetails',
      'mnWarmupProgress',
      'mnElementCrane',
      'mnSortableTable'
    ])
    .controller('mnBucketsController', mnBucketsController);

  function mnBucketsController($scope, mnBucketsService, mnHelper, mnPoolDefault, mnPromiseHelper, mnPoller, $uibModal, $rootScope, $interval) {
    var vm = this;

    var poolDefault = mnPoolDefault.latestValue();

    vm.isCreateNewDataBucketDisabled = isCreateNewDataBucketDisabled;
    vm.isBucketCreationWarning = isBucketCreationWarning;
    vm.isMaxBucketCountWarning = isMaxBucketCountWarning;
    vm.areThereCreationWarnings = areThereCreationWarnings;
    vm.addBucket = addBucket;

    vm.maxBucketCount = poolDefault.value.maxBucketCount;

    activate();

    function activate() {
      var pull = $interval(function () {
        $rootScope.$broadcast("reloadBucketStats");
      }, 3000);

      $rootScope.$broadcast("reloadBucketStats");

      $scope.$on('$destroy', function () {
        $interval.cancel(pull);
      });
    }

    function isCreateNewDataBucketDisabled() {
      return !$scope.buckets || !$scope.buckets.details || areThereCreationWarnings();
    }
    function isBucketCreationWarning() {
      return poolDefault.value.rebalancing;
    }
    function isMaxBucketCountWarning() {
      return (($scope.buckets && $scope.buckets.details) || []).length >= poolDefault.value.maxBucketCount;
    }
    function areThereCreationWarnings() {
      return isMaxBucketCountWarning() || isBucketCreationWarning();
    }
    function addBucket() {
      mnPromiseHelper(vm, mnPoolDefault.getFresh())
        .onSuccess(function (poolDefault) {
          if (poolDefault.storageTotals.ram.quotaTotal === poolDefault.storageTotals.ram.quotaUsed) {
            $uibModal.open({
              templateUrl: 'app/mn_admin/mn_bucket_full_dialog.html'
            });
          } else {
            !areThereCreationWarnings() && $uibModal.open({
              templateUrl: 'app/mn_admin/mn_buckets_details_dialog.html',
              controller: 'mnBucketsDetailsDialogController as bucketsDetailsDialogCtl',
              resolve: {
                bucketConf: function (mnBucketsDetailsDialogService) {
                  return mnBucketsDetailsDialogService.getNewBucketConf();
                },
                autoCompactionSettings: function (mnSettingsAutoCompactionService) {
                  return mnSettingsAutoCompactionService.getAutoCompaction(true);
                }
              }
            });
          }
        });
    }
  }
})();
