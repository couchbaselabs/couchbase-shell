(function () {
  "use strict";

  angular
    .module('mnGsi')
    .directive('mnGsiTable', mnGsiTableDirective);

  function mnGsiTableDirective(mnHelper) {
    var mnGsiTable = {
      restrict: 'EA',
      scope: {
        list: "=",
        hideColumn: "@",
        filterField: "=",
        rbac: "=",
        pools: "=",
        nodeName: "@?"
      },
      controller: mnGsiTableController,
      controllerAs: "mnGsiTableCtl",
      templateUrl: 'app/mn_admin/mn_gsi_table_directive.html'
    };

    return mnGsiTable;

    function mnGsiTableController($scope) {
      var vm = this;
      vm.generateIndexId = generateIndexId;
      vm.getStatusClass = getStatusClass;
      vm.getStatusDescription = getStatusDescription;

      mnHelper.initializeDetailsHashObserver(vm, 'openedIndex', 'app.admin.gsi');


      function generateIndexId(row, partitionHost) {
        return (row.id.toString() + (row.instId || "")) +
          (row.hosts ? row.hosts.join() : "") +
          ($scope.nodeName || "");
      }

      function getStatusClass(row) {
        row = row || {};
        if (row.stale) { //MB-36247
          return 'dynamic_warmup';
        }
        switch (row.status) {
          case 'Ready': return 'dynamic_healthy';
          case 'Not Available': return 'dynamic_unhealthy';
          case 'Error': return 'dynamic_unhealthy';
          case 'Paused': return 'dynamic_unhealthy';
          case 'Replicating':
          case 'Created':
          case 'Building':
          case 'Warmup':
          case 'Created (Upgrading)':
          case 'Created (Downgrading)':
          case 'Building (Upgrading)':
          case 'Building (Downgrading)': return 'dynamic_warmup';
          default: return 'dynamic_warmup';
        }
      }
      function getStatusDescription(row) {
        row = row || {};
        switch (row.status) {
          case 'Created': return 'Index definition has been saved. Use Build Index to build the index. It is NOT serving scan requests yet.';
          case 'Building': return 'Index is currently building. It is NOT serving scan requests yet.';
          case 'Ready': return 'Index is ready to serve scan requests.';
          case 'Replicating': return 'Index is being replicated as part of a Rebalance or Alter Index operation. It is NOT serving scan requests until replication is complete.';
          case 'Paused': return 'Index is not ingesting new mutations as allocated memory has been completely used.';
          case 'Warmup': return 'Index is being loaded from persisted on-disk snapshot after indexer process restart. It is NOT serving scan requests yet.';
          case 'Error': return 'Index is in an error state and cannot be used in scan operations.';
          case 'Created (Upgrading)': return 'Index definition has been upgraded from Legacy storage engine to Standard GSI. It is NOT serving scan requests yet.';
          case 'Created (Downgrading)': return 'Index definition has been downgraded from Standard GSI to Legacy storage engine. It is NOT serving scan requests yet.'  ;
          case 'Building (Upgrading)': return 'Index is building after upgrade from Legacy storage engine to Standard GSI. It is NOT serving scan requests yet.';
          case 'Building (Downgrading)': return 'Index is building after downgrade from Standard GSI to Legacy storage engine. It is NOT serving scan requests yet.';
          case 'Not Available': return 'Index not available.';
        }
      }

    }
  }
})();
