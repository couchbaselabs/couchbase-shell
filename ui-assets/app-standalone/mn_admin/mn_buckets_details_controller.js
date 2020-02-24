(function () {
  "use strict";

  angular
    .module('mnBuckets')
    .controller('mnBucketsDetailsController', mnBucketsDetailsController);

  function mnBucketsDetailsController($scope, mnBucketsDetailsService, mnTasksDetails, mnPromiseHelper, mnSettingsAutoCompactionService, mnCompaction, $uibModal, mnBytesToMBFilter, mnBucketsDetailsDialogService, mnPoller, mnHelper) {
      var vm = this;
      vm.editBucket = editBucket;
      vm.deleteBucket = deleteBucket;
      vm.flushBucket = flushBucket;
      vm.registerCompactionAsTriggeredAndPost = registerCompactionAsTriggeredAndPost;
      vm.getGuageConfig = getGuageConfig;
      vm.getEndings = mnHelper.getEndings;

      var compactionTasks;
      var warmUpTasks;

      activate();


      function activate() {
        compactionTasks = new mnPoller($scope, function () {
          return mnBucketsDetailsService.getCompactionTask($scope.bucket);
        })
        .subscribe("compactionTasks", vm)
        .reloadOnScopeEvent("mnTasksDetailsChanged")
        .cycle();

        $scope.$watch('bucket', function () {
          compactionTasks.reload();
          mnPromiseHelper(vm, mnBucketsDetailsService.doGetDetails($scope.bucket)).applyToScope("bucketDetails");
        });

        $scope.$watchGroup(['bucket', 'adminCtl.tasks'], function (values) {
          vm.warmUpTasks = mnBucketsDetailsService.getWarmUpTasks(values[0], values[1]);
        });
      }

      $scope.$watch("bucketsDetailsCtl.bucketDetails", getBucketRamGuageConfig);
      function getBucketRamGuageConfig(details) {
        vm.bucketRamGuageConfig = mnBucketsDetailsService.getBucketRamGuageConfig(details && {
          total: details.basicStats.storageTotals.ram.quotaTotalPerNode * details.nodes.length,
          thisAlloc: details.quota.ram,
          otherBuckets: details.basicStats.storageTotals.ram.quotaUsedPerNode * details.nodes.length - details.quota.ram
        });
      }
      function getGuageConfig(details) {
        if (!details) {
          return;
        }
        return mnBucketsDetailsService.getGuageConfig(
          details.basicStats.storageTotals.hdd.total,
          details.basicStats.diskUsed,
          details.basicStats.storageTotals.hdd.usedByData - details.basicStats.diskUsed,
          details.basicStats.storageTotals.hdd.used - details.basicStats.storageTotals.hdd.usedByData
        );
      }
      function editBucket() {
        $uibModal.open({
          templateUrl: 'app/mn_admin/mn_buckets_details_dialog.html',
          controller: 'mnBucketsDetailsDialogController as bucketsDetailsDialogCtl',
          resolve: {
            bucketConf: function () {
              return mnBucketsDetailsDialogService.reviewBucketConf(vm.bucketDetails);
            },
            autoCompactionSettings: function () {
              if (vm.bucketDetails.autoCompactionSettings === undefined) {
                return;
              }
              return !vm.bucketDetails.autoCompactionSettings ?
                      mnSettingsAutoCompactionService.getAutoCompaction(true) :
                      mnSettingsAutoCompactionService.prepareSettingsForView(vm.bucketDetails);
            }
          }
        });
      }
      function deleteBucket(bucket) {
        $uibModal.open({
          templateUrl: 'app/mn_admin/mn_buckets_delete_dialog.html',
          controller: 'mnBucketsDeleteDialogController as bucketsDeleteDialogCtl',
          resolve: {
            bucket: function () {
              return bucket;
            }
          }
        });
      }
      function flushBucket(bucket) {
        $uibModal.open({
          templateUrl: 'app/mn_admin/mn_buckets_flush_dialog.html',
          controller: 'mnBucketsFlushDialogController as bucketsFlushDialogCtl',
          resolve: {
            bucket: function () {
              return bucket;
            }
          }
        });
      }
      function registerCompactionAsTriggeredAndPost(url, disableButtonKey) {
        vm.compactionTasks[disableButtonKey] = true;
        mnPromiseHelper(vm, mnCompaction.registerAsTriggeredAndPost(url))
          .onSuccess(function () {
            compactionTasks.reload()
          });
      };
    }
})();
