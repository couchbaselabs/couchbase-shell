(function () {
  "use strict";

  angular
    .module("mnDocuments", [
      "mnDocumentsListService",
      "mnDocumentsEditingService",
      "mnPromiseHelper",
      "mnFilter",
      "mnFilters",
      "ui.router",
      "ui.bootstrap",
      "ui.codemirror",
      "mnSpinner",
      "ngMessages",
      "mnPoll",
      "mnElementCrane"
    ])
    .controller("mnDocumentsController", mnDocumentsController);

  function mnDocumentsController($scope, mnDocumentsListService, mnPromiseHelper, $state, mnPoller) {
    var vm = this;

    vm.onSelectBucketName = onSelectBucketName;
    vm.currentBucketName = $state.params.bucket;

    function onSelectBucketName(selectedBucket) {
      $state.go('^.list', {
        bucket: selectedBucket,
        pageNumber: 0
      });
    }
  }
})();
