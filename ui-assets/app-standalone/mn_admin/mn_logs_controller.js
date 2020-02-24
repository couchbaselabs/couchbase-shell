(function () {
  "use strict";

  angular.module('mnLogs', [
    'mnLogsService',
    'mnPromiseHelper',
    'mnPoll',
    'mnSpinner',
    'mnFilters',
    'mnElementCrane',
    'mnSearch',
    'mnSortableTable',
    'mnLogRedactionService',
    'mnSelectableNodesList',
    'mnGroupsService'
  ]).controller('mnLogsController', mnLogsController);

  function mnLogsController($scope, mnHelper, mnLogsService) {
    var vm = this;
  }
})();
