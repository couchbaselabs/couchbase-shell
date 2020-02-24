(function () {
  "use strict";

  angular
    .module('mnSearch', [])
    .directive('mnSearch', mnSearchDirective);

  function mnSearchDirective() {

    var mnSearch = {
      restrict: 'AE',
      scope: {
        mnSearch: "=",
        mnPlaceholder: "@",
        mnHideButton: "=",
        mnDisabled: "="
      },
      templateUrl: 'app/components/directives/mn_search/mn_search.html',
      controller: controller,
      controllerAs: "mnSearchCtl",
      bindToController: true
    };

    return mnSearch;

    function controller() {
      var vm = this;
      vm.hideFilter = hideFilter;
      vm.showFilter = showFilter;

      function hideFilter() {
        vm.mnSearch = "";
        vm.showFilterFlag = false;
      }
      function showFilter() {
        vm.showFilterFlag = true;
        vm.focusFilterField = true;
      }
    }
  }
})();
