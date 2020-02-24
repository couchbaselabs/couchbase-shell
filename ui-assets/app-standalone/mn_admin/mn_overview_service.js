(function () {
  "use strict"

  angular
    .module('mnOverviewService', [
      'mnPoolDefault',
      'mnServersService'
    ])
    .factory('mnOverviewService', mnOverviewServiceFactory);

  function mnOverviewServiceFactory($http, mnPoolDefault, mnServersService) {

    var mnOverviewService = {
      getStats: getStats,
      getOverviewConfig: getOverviewConfig,
      getServices: getServices
    };

    var processPlotOptions = function (plotOptions, plotDatas) {
      var firstData = plotDatas[0];
      var t0 = firstData[0][0];
      var t1 = this.lastSampleTime;
      if (t1 - t0 < 300000) {
        plotOptions.xaxis.ticks = [t0, t1];
        plotOptions.xaxis.tickSize = [null, "minute"];
      }
      return plotOptions;
    };

    var ramOverviewConfigBase = {
      topRight: {
        name: 'total quota'
      },
      items: [{
        name: 'in use'
      }, {
        name: 'unused quota'
      }, {
        name: 'unallocated'
      }]
    };

    var hddOverviewConfigBase = {
      topRight: {
        name: 'usable free space'
      },
      items: [{
        name: 'in use by couchbase'
      }, {
        name: 'other data'
      }, {
        name: "free"
      }]
    };

    return mnOverviewService;


    function getNodesByService(service, nodes) {
      var nodes2 = _.filter(nodes.allNodes, function (node) {
        return _.indexOf(node.services, service) > -1;
      });

      return mnServersService.addNodesByStatus(nodes2);
    }

    function getServices() {
      return mnServersService.getNodes().then(function (nodes) {
        var rv = {
          kv: getNodesByService("kv", nodes),
          index: getNodesByService("index", nodes),
          n1ql: getNodesByService("n1ql", nodes),
          fts: getNodesByService("fts", nodes),
          all: nodes
        };
        if (mnPoolDefault.export.isEnterprise) {
          rv.cbas = getNodesByService("cbas", nodes);
          rv.eventing = getNodesByService("eventing", nodes);
        }
        return rv;
      });
    }

    function getStats() {
      return $http({
        url: '/pools/default/overviewStats',
        method: "GET"
      }).then(function (statsResponse) {
        var stats = statsResponse.data;
        var now = new Date().valueOf();
        var tstamps = stats.timestamp || [];
        var interval = tstamps[tstamps.length - 1] - tstamps[0];
        var breakInterval = (tstamps.length > 1) ? (interval / Math.min(tstamps.length / 2, 30)) : undefined;

        var options = {
          lastSampleTime: now,
          breakInterval: breakInterval,
          processPlotOptions: processPlotOptions
        };

        return {
          opsGraphConfig: {
            stats: stats['ops'],
            tstamps: tstamps,
            options: options
          },
          readsGraphConfig: {
            stats: stats['ep_bg_fetched'],
            tstamps: tstamps,
            options: options
          }
        };
      });
    }
    function getOverviewConfig() {
      return mnPoolDefault.get().then(function (poolsDetails) {
        var details = poolsDetails;
        var rv = {};

        (function () {
          var ram = details.storageTotals.ram;
          var usedQuota = ram.usedByData;
          var bucketsQuota = ram.quotaUsed;
          var quotaTotal = ram.quotaTotal;

          var ramOverviewConfig = _.clone(ramOverviewConfigBase, true);

          ramOverviewConfig.topRight.value = bucketsQuota;
          ramOverviewConfig.items[0].value = usedQuota;
          ramOverviewConfig.items[1].value = bucketsQuota - usedQuota;
          ramOverviewConfig.items[2].value = Math.max(quotaTotal - bucketsQuota, 0);

          if (ramOverviewConfig.items[1].value < 0) {
            ramOverviewConfig.items[0].value = bucketsQuota;
            ramOverviewConfig.items[1] = {
              name: 'overused',
              value: usedQuota - bucketsQuota
            };
            ramOverviewConfig.topRight = {
              name: 'cluster quota',
              value: quotaTotal
            };
            if (usedQuota < quotaTotal) {
              ramOverviewConfig.items[2] = {
                name: 'available',
                value: quotaTotal - usedQuota
              };
            } else {
              ramOverviewConfig.items.pop();
            }
          }

          rv.ramOverviewConfig = ramOverviewConfig;
        })();

        ;(function () {
          var hdd = details.storageTotals.hdd;

          var usedSpace = hdd.usedByData;
          var total = hdd.total;
          var other = hdd.used - usedSpace;
          var free = hdd.free;

          var hddOverviewConfig = _.clone(hddOverviewConfigBase, true);

          hddOverviewConfig.topRight.value = free;
          hddOverviewConfig.items[0].value = usedSpace;
          hddOverviewConfig.items[1].value = other;
          hddOverviewConfig.items[2].value = total - other - usedSpace;

          rv.hddOverviewConfig = hddOverviewConfig;
        })();

        return rv;
      });
    }
  }
})();
