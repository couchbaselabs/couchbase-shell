(function () {
  "use strict";

  angular
    .module("mnFilter", [])
    .directive("mnFilter", mnFilterDirective);

  function mnFilterDirective($window) {
    var mnFilter = {
      restrict: "A",
      scope: {
        config: "=",
        mnDisabled: "=",
        onClose: "&",
        onOpen: "&",
        onReset: "&"
      },
      templateUrl: "app/components/directives/mn_filter/mn_filter.html",
      controller: mnFilterController,
      controllerAs: "mnFilterCtl",
      bindToController: true
    };

    return mnFilter;

    function mnFilterController($scope) {
      var vm = this;

      vm.togglePopup = togglePopup;


      function togglePopup(open) {
        vm.showPopup = open;
        if (vm.showPopup) {
          vm.onOpen && vm.onOpen();
        } else {
          vm.onClose && vm.onClose();
        }
      }
    }
  }
})();
