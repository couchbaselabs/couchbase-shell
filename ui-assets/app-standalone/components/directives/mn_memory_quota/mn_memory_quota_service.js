(function () {
  "use strict";

  angular
    .module('mnMemoryQuotaService', [
      'mnPoolDefault',
      'mnHelper'
    ])
    .factory('mnMemoryQuotaService', mnMemoryQuotaServiceFactory);

  function mnMemoryQuotaServiceFactory($http, $window, mnPoolDefault, mnHelper, IEC) {
    var mnMemoryQuotaService = {
      prepareClusterQuotaSettings: prepareClusterQuotaSettings,
      isOnlyOneNodeWithService: isOnlyOneNodeWithService,
      memoryQuotaConfig: memoryQuotaConfig,
      getFirstTimeAddedServices: getFirstTimeAddedServices,
      handleAltAndClick: handleAltAndClick
    };

    return mnMemoryQuotaService;

    function prepareClusterQuotaSettings(currentPool, displayedServices, calculateMaxMemory, calculateTotal) {
      var ram = currentPool.storageTotals.ram;
      if (calculateMaxMemory === undefined) {
        calculateMaxMemory = displayedServices.kv;
      }
      var rv = {
        calculateTotal: calculateTotal,
        displayedServices: displayedServices,
        minMemorySize: Math.max(256, Math.floor(ram.quotaUsedPerNode / IEC.Mi)),
        totalMemorySize: Math.floor(ram.total/IEC.Mi),
        memoryQuota: Math.floor(ram.quotaTotalPerNode/IEC.Mi)
      };

      rv.indexMemoryQuota = currentPool.indexMemoryQuota || 256;
      rv.ftsMemoryQuota = currentPool.ftsMemoryQuota || 256;

      if (currentPool.compat.atLeast55 && mnPoolDefault.export.isEnterprise) {
        rv.cbasMemoryQuota = currentPool.cbasMemoryQuota || 256;
        rv.eventingMemoryQuota = currentPool.eventingMemoryQuota || 256;
      }
      if (calculateMaxMemory) {
        rv.maxMemorySize = mnHelper.calculateMaxMemorySize(ram.total / IEC.Mi);
      } else {
        rv.maxMemorySize = false;
      }

      return rv;
    }
    function getFirstTimeAddedServices(interestedServices, selectedServices, allNodes) {
      var rv = {
        count: 0
      };
      angular.forEach(interestedServices, function (interestedService) {
        if (selectedServices[interestedService] && mnMemoryQuotaService.isOnlyOneNodeWithService(allNodes, selectedServices, interestedService)) {
          rv[interestedService] = true;
          rv.count++;
        }
      });
      return rv;
    }
    function isOnlyOneNodeWithService(nodes, services, service, isTakenIntoAccountPendingEject) {
      var nodesCount = 0;
      var indexExists = _.each(nodes, function (node) {
        nodesCount += (_.indexOf(node.services, service) > -1 && !(isTakenIntoAccountPendingEject && node.pendingEject));
      });
      return nodesCount === 1 && services && (angular.isArray(services) ? (_.indexOf(services, service) > -1) : services[service]);
    }
    function memoryQuotaConfig(displayedServices, calculateMaxMemory, calculateTotal) {
      return mnPoolDefault.get().then(function (poolsDefault) {
        return mnMemoryQuotaService.prepareClusterQuotaSettings(poolsDefault, displayedServices, calculateMaxMemory, calculateTotal);
      });
    }

    function toggleServices(service, bool, config) {
      _.forEach(config.services.model, function (_, service1) {
        if ((config.services.disabled && config.services.disabled[service1]) ||
            (config.displayedServices && !config.displayedServices[service1])
           ) {
          return;
        }
        config.services.model[service1] = bool;
      });
    }

    function isThereOther(service, config) {
      return _.keys(config.services.model)
        .some(function (service1) {
          return (!config.displayedServices || config.displayedServices[service1]) &&
            (!config.services.disabled || !config.services.disabled[service1]) &&
            config.services.model[service1] &&
            service1 !== service;
        });
    }

    function handleAltAndClick(service, config) {
      if (!$window.event.altKey) {
        return;
      }
      if (isThereOther(service, config)) {
        toggleServices(service, false, config);
      } else {
        toggleServices(service, true, config);
      }
      config.services.model[service] = true;
    }

  }
})();
