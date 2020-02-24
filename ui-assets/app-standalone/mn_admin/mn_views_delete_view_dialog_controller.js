(function () {
  "use strict";

  angular
    .module("mnViews")
    .controller("mnViewsDeleteViewDialogController", mnViewsDeleteViewDialogController);

  function mnViewsDeleteViewDialogController($scope, $state, $uibModalInstance, mnPromiseHelper, mnViewsListService, currentDdocName, currentViewName, isSpatial) {
    var vm = this;
    vm.currentDdocName = currentDdocName;
    vm.currentViewName = currentViewName;
    vm.maybeSpatial = isSpatial ? 'Spatial' : '';
    vm.doDelete = doDelete;

    function doDelete() {
      var url = mnViewsListService.getDdocUrl($state.params.bucket, currentDdocName);

      var promise = mnViewsListService.getDdoc(url).then(function (presentDdoc) {
        delete presentDdoc.json[isSpatial ? 'spatial' : 'views'][currentViewName];
        return mnViewsListService.createDdoc(url, presentDdoc.json);
      });

      mnPromiseHelper(vm, promise, $uibModalInstance)
        .showGlobalSpinner()
        .closeFinally()
        .broadcast("reloadViewsPoller")
        .showGlobalSuccess("View deleted successfully!");
    };
  }
})();
