(function () {
  "use strict";

  angular
    .module("mnViews")
    .controller("mnViewsEditingResultController", mnViewsEditingResultController);

  function mnViewsEditingResultController($scope, $state, mnPromiseHelper, mnViewsEditingService, viewsPerPageLimit) {
    var vm = this;
    var filterConfig = {};
    if ($state.params.isSpatial) {
      filterConfig.items = {
        stale: true,
        connectionTimeout: true,
        bbox: true,
        startRange: true,
        endRange: true
      };
    } else {
      filterConfig.items = {
        stale: true,
        connectionTimeout: true,
        descending: true,
        startkey: true,
        endkey: true,
        startkeyDocid: true,
        endkeyDocid: true,
        group: true,
        groupLevel: true,
        inclusiveEnd: true,
        key: true,
        keys: true,
        reduce: true
      };
    }

    vm.filterConfig = filterConfig;
    vm.onFilterClose = onFilterClose;
    vm.onFilterOpen = onFilterOpen;
    vm.onFilterReset = onFilterReset;
    vm.isPrevDisabled = isPrevDisabled;
    vm.isNextDisabled = isNextDisabled;
    vm.nextPage = nextPage;
    vm.prevPage = prevPage;
    vm.loadSampleDocument = loadSampleDocument;
    vm.generateViewHref = generateViewHref;
    vm.getFilterParamsAsString = mnViewsEditingService.getFilterParamsAsString;
    vm.activate = activate;

    filterConfig.params = mnViewsEditingService.getFilterParams();

    if ($state.params.activate) {
      $state.go('^.result', {
        activate: false
      });
      activate();
    }

    function onFilterReset() {
      filterConfig.params = mnViewsEditingService.getInitialViewsFilterParams($state.params.isSpatial);
    }

    function generateViewHref() {
      return $scope.viewsEditingCtl.state &&
            ($scope.viewsEditingCtl.state.capiBase +
              mnViewsEditingService.buildViewUrl($state.params) +
              mnViewsEditingService.getFilterParamsAsString());
    }

    function nextPage() {
      $state.go('^.result', {
        pageNumber: $state.params.pageNumber + 1,
        activate: true
      });
    }
    function prevPage() {
      var prevPage = $state.params.pageNumber - 1;
      prevPage = prevPage < 0 ? 0 : prevPage;
      $state.go('^.result', {
        pageNumber: prevPage,
        activate: true
      });
    }
    function loadSampleDocument(id) {
      $state.go('^.result', {
        sampleDocumentId: id
      });
    }
    function isEmptyState() {
      return !vm.state || vm.state.isEmptyState;
    }
    function isPrevDisabled() {
      return isEmptyState() || vm.viewLoading || $state.params.pageNumber <= 0;
    }
    function isNextDisabled() {
      return isEmptyState() || vm.viewLoading || !vm.state.rows || vm.state.rows.length < viewsPerPageLimit || $state.params.pageNumber >= 15;
    }
    function onFilterClose() {
      var params = filterConfig.params
      if (params.group === false) {
        delete params.group;
      }
      if (params.descending === false) {
        delete params.descending;
      }
      $state.go('^.result', {
        viewsParams: JSON.stringify(params)
      });
      $scope.viewsEditingCtl.isFilterOpened = false;
    }
    function onFilterOpen() {
      $scope.viewsEditingCtl.isFilterOpened = true;
    }
    function activate() {
      mnPromiseHelper(vm, mnViewsEditingService.getViewResult($state.params))
        .showSpinner()
        .catchErrors()
        .applyToScope("state");
    }
  }
})();
