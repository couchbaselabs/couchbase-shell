(function () {
  "use strict";

  angular
    .module('mnBuckets')
    .directive('mnBucketsList', mnBucketsList);

  function mnBucketsList(mnHelper) {
    var mnBucketsListDirective = {
      restrict: 'A',
      scope: {
        buckets: '=',
        rbac: "=",
        poolDefault: "=",
        adminCtl: "="
      },
      templateUrl: 'app/mn_admin/mn_buckets_list.html',
      controller: controller,
      controllerAs: "bucketsListCtl"
    };

    return mnBucketsListDirective;

    function controller() {
      var vm = this;
      mnHelper.initializeDetailsHashObserver(vm, 'openedBucket', 'app.admin.buckets');
    }
  }
})();
