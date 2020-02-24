(function () {
  "use strict";

  angular.module('mnSettingsAutoFailover', [
    'mnSettingsAutoFailoverService',
    'mnHelper',
    'mnPromiseHelper',
    "mnSettingsClusterService"
  ]).controller('mnSettingsAutoFailoverController', mnSettingsAutoFailoverController);

  function mnSettingsAutoFailoverController($scope, $q, mnHelper, mnPromiseHelper, mnSettingsAutoFailoverService, mnPoolDefault, mnSettingsClusterService) {
    var vm = this;

    mnSettingsClusterService.registerSubmitCallback(submit);

    activate();

    function getAutoFailoverSettings() {
      var settings = {
        enabled: vm.autoFailoverSettings.enabled,
        timeout: vm.autoFailoverSettings.timeout
      };
      if (mnPoolDefault.export.compat.atLeast55 &&
          mnPoolDefault.export.isEnterprise) {
        settings.failoverOnDataDiskIssues = vm.autoFailoverSettings.failoverOnDataDiskIssues;
        settings.failoverServerGroup = vm.autoFailoverSettings.failoverServerGroup;
        settings.maxCount = vm.autoFailoverSettings.maxCount;
      }
      if (mnPoolDefault.export.compat.atLeast65 &&
          mnPoolDefault.export.isEnterprise) {
        settings.canAbortRebalance = vm.autoFailoverSettings.canAbortRebalance;
      }
      return settings;
    }

    function getReprovisionSettings() {
      return {
        enabled: vm.reprovisionSettings.enabled,
        maxNodes: vm.reprovisionSettings.max_nodes
      };
    }

    function watchOnSettings(method, dataFunc) {
      return function () {
        if (!$scope.rbac.cluster.settings.write) {
          return;
        }
        mnPromiseHelper(vm, mnSettingsAutoFailoverService[method](dataFunc(), {just_validate: 1}))
          .catchErrorsFromSuccess(function (rv) {
            vm[method + "Errors"] = rv;
            $scope.settingsClusterCtl[method + "Errors"] = rv;
          });
      }
    }

    function activate() {
      mnPromiseHelper(vm, mnSettingsAutoFailoverService.getAutoReprovisionSettings())
        .applyToScope(function (resp) {
          vm.reprovisionSettings = resp.data;

          $scope.$watch(
            'settingsAutoFailoverCtl.reprovisionSettings',
            _.debounce(watchOnSettings("postAutoReprovisionSettings", getReprovisionSettings),
                       500, {leading: true}), true);
        });

      mnPromiseHelper(vm,
        mnSettingsAutoFailoverService.getAutoFailoverSettings())
        .applyToScope(function (resp) {
          vm.autoFailoverSettings = resp;

          $scope.$watch(
            'settingsAutoFailoverCtl.autoFailoverSettings',
            _.debounce(watchOnSettings("saveAutoFailoverSettings", getAutoFailoverSettings),
                       500, {leading: true}), true);
        });
    }

    function submit() {
      var queries = [
        mnPromiseHelper(vm, mnSettingsAutoFailoverService.saveAutoFailoverSettings(getAutoFailoverSettings()))
          .catchErrors(function (resp) {
            vm.saveAutoFailoverSettingsErrors = resp && {timeout: resp};
          })
          .getPromise(),

        mnPromiseHelper(vm, mnSettingsAutoFailoverService.postAutoReprovisionSettings(getReprovisionSettings()))
          .catchErrors(function (resp) {
            vm.postAutoReprovisionSettingsErrors = resp && {maxNodes: resp};
          })
          .getPromise()
      ];

      return $q.all(queries);
    };
  }
})();
