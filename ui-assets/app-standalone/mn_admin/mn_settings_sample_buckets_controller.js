(function () {
  "use strict";

  angular
    .module("mnSettingsSampleBuckets", [
      "mnSettingsSampleBucketsService",
      "mnPromiseHelper",
      "mnElementCrane"
    ])
    .controller("mnSettingsSampleBucketsController", mnSettingsSampleBucketsController);

  function mnSettingsSampleBucketsController($scope, mnSettingsSampleBucketsService, mnPromiseHelper) {
    var vm = this;
    vm.selected = {};
    vm.isCreateButtonDisabled = isCreateButtonDisabled;
    vm.installSampleBuckets = installSampleBuckets;
    vm.isAnyBucketSelected = isAnyBucketSelected;

    activate();
    function getState(selected) {
      return mnPromiseHelper(vm, mnSettingsSampleBucketsService.getSampleBucketsState(selected || vm.selected)).applyToScope("state");
    }
    function doGetState() {
      getState();
    }
    function activate() {
      getState().showSpinner();
      $scope.$watch("settingsSampleBucketsCtl.selected", function (value, oldValue) {
        if (value !== oldValue) {
          getState(value);
        }
      }, true);
      $scope.$on("reloadBucketStats", doGetState);
      $scope.$on("nodesChanged", doGetState);
      $scope.$on("reloadTasksPoller", doGetState);
    }


    function installSampleBuckets() {
      mnPromiseHelper(vm, mnSettingsSampleBucketsService.installSampleBuckets(vm.selected))
        .showGlobalSpinner()
        .catchGlobalErrors()
        .reloadState("app.admin.settings")
        .showGlobalSuccess("Task added successfully!");
    }

    function isAnyBucketSelected() {
      return _.keys(_.pick(vm.selected, _.identity)).length;
    }

    function isCreateButtonDisabled() {
      return vm.viewLoading || vm.state &&
             (_.chain(vm.state.warnings).values().some().value() ||
             !vm.state.available.length) ||
             !isAnyBucketSelected();
    }

  }
})();
