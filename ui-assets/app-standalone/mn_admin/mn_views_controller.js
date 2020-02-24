(function () {
  "use strict";

  angular
    .module("mnViews", [
      'mnViewsListService',
      'mnViewsEditingService',
      'mnCompaction',
      'mnHelper',
      'mnPromiseHelper',
      'mnPoll',
      'mnFilter',
      'ui.select',
      'ui.router',
      'ui.bootstrap',
      'ngSanitize',
      'mnPoolDefault',
      'ngMessages'
    ])
    .controller("mnViewsController", mnViewsController);

    function mnViewsController($scope, $state, mnPoller, $q, mnViewsListService, mnPoolDefault) {

      var vm = this;
      vm.onSelectBucket = onSelectBucket;
      vm.mnPoolDefault = mnPoolDefault.latestValue();
      vm.ddocsLoading = true;
      vm.currentBucketName = $state.params.bucket;

      function onSelectBucket(selectedBucket) {
        $state.go('^.list', {bucket: selectedBucket});
      }
    }
})();
