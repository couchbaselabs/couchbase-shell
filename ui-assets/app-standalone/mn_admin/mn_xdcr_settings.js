(function () {
  "use strict";

  angular
    .module('mnXDCR')
    .directive('mnXdcrSettings', mnXdcrSettingsDirective);

  function mnXdcrSettingsDirective($http, mnPromiseHelper, mnXDCRService) {
      var mnXdcrSettings = {
        restrict: 'A',
        scope: {
          settings: '=mnXdcrSettings',
          mnPoolDefault: "=",
          mnPools: "=",
          mnIsEditing: "=?"
        },
        isolate: false,
        replace: true,
        templateUrl: 'app/mn_admin/mn_xdcr_settings.html',
        controller: controller,
        controllerAs: "xdcrSettingsCtl",
        bindToController: true
      };

      return mnXdcrSettings;

      function controller($scope) {
        var vm = this;
        $scope.$watch('xdcrSettingsCtl.settings', function (settings) {
          mnPromiseHelper(vm, mnXDCRService.postSettingsReplications(settings, true)).catchErrors();
        }, true);
      }
    }
})();
