(function () {
  "use strict";

  angular
    .module('mnAnalyticsService', [
      'mnBucketsService',
      'mnServersService',
      'mnFilters',
      'mnStatisticsNewService'
    ])
    .factory('mnAnalyticsService', mnAnalyticsServiceFactory);

  function mnAnalyticsServiceFactory($http, $q, mnBucketsService, mnServersService, mnCloneOnlyDataFilter, mnFormatQuantityFilter, mnParseHttpDateFilter, timeUnitToSeconds, mnStatisticsNewService) {
    var mnAnalyticsService = {
      getStats: getStats,
      doGetStats: doGetStats,
      prepareNodesList: prepareNodesList
    };

    return mnAnalyticsService;

    function restoreOpsBlock(prevSamples, samples, keepCount) {
      var prevTS = prevSamples.timestamp;
      if (samples.timestamp && samples.timestamp.length == 0) {
        // server was unable to return any data for this "kind" of
        // stats
        if (prevSamples && prevSamples.timestamp && prevSamples.timestamp.length > 0) {
          return prevSamples;
        }
        return samples;
      }
      if (prevTS == undefined ||
          prevTS.length == 0 ||
          prevTS[prevTS.length-1] != samples.timestamp[0]) {
        return samples;
      }
      var newSamples = {};
      for (var keyName in samples) {
        var ps = prevSamples[keyName];
        if (!ps) {
          ps = [];
          ps.length = keepCount;
        }
        newSamples[keyName] = ps.concat(samples[keyName].slice(1)).slice(-keepCount);
      }
      return newSamples;
    }
    function maybeApplyDelta(prevValue, value) {
      var stats = value.stats;
      var prevStats = prevValue.stats || {};
      for (var kind in stats) {
        var newSamples = restoreOpsBlock(prevStats[kind],
                                         stats[kind],
                                         value.samplesCount + 1);
        stats[kind] = newSamples;
      }
      return value;
    }
    function prepareNodesList(params) {
      return mnServersService.getNodes().then(function (nodes) {
        var rv = {};
        rv.nodesNames = _(nodes.active).filter(function (node) {
          return !(node.clusterMembership === 'inactiveFailed') && !(node.status === 'unhealthy');
        }).pluck("hostname").value();
        rv.nodesNames.unshift("All Server Nodes (" + rv.nodesNames.length + ")");
        rv.nodesNames.selected = params.statsHostname === "all" ? rv.nodesNames[0] : params.statsHostname;
        return rv;
      });
    }
    function getStats(params) {
      var isSpecificStat = !!params.$stateParams.specificStat;
      return mnAnalyticsService.doGetStats(params).then(function (resp) {
        var queries = [
          $q.when(resp)
        ];
        queries.push(isSpecificStat ? $q.when({
          data: resp.data.directory.value,
          origTitle: resp.data.directory.origTitle
        }) : mnStatisticsNewService.getStatsDirectory(params.$stateParams.bucket));
        return $q.all(queries).then(function (data) {
          return prepareAnaliticsState(data, params);
        });
      }, function (resp) {
        switch (resp.status) {
        case 0:
        case -1: return $q.reject(resp);
        case 404: return !params.$stateParams.bucket ? {status: "_404"} : resp;
        default: return resp;
        }
      });
    }
    function doGetStats(params, mnHttpParams) {
      var reqParams = {
        zoom: params.$stateParams.zoom,
        bucket: params.$stateParams.bucket
      };
      if (params.$stateParams.specificStat) {
        reqParams.statName = params.$stateParams.specificStat;
      } else {
        if (params.$stateParams.statsHostname !== "all") {
          reqParams.node = params.$stateParams.statsHostname;
        }
      }
      if (params.previousResult && !params.previousResult.status) {
        reqParams.haveTStamp = params.previousResult.stats.lastTStamp;
      }
      return $http({
        url: '/_uistats',
        method: 'GET',
        params: reqParams,
        mnHttp: mnHttpParams
      });
    }
    function prepareAnaliticsState(data, params) {
      var stats = mnCloneOnlyDataFilter(data[0].data);
      var statDesc = mnCloneOnlyDataFilter(data[1].data);
      var samples = {};
      var rv = {};
      if (params.previousResult && !params.previousResult.status) {
        stats = maybeApplyDelta(params.previousResult.stats, stats);
      }

      angular.forEach(stats.stats, function (subSamples, subName) {
        var timestamps = subSamples.timestamp;
        for (var k in subSamples) {
          if (k == "timestamp") {
            continue;
          }
          samples[k] = subSamples[k];
          samples[k].timestamps = timestamps;
        }
      });

      stats.serverDate = mnParseHttpDateFilter(data[0].headers('date')).valueOf();
      stats.clientDate = (new Date()).valueOf();

      var statsByName = {};
      var breakInterval = stats.interval * 2.5;
      var timeOffset = stats.clientDate - stats.serverDate;
      var zoomMillis = timeUnitToSeconds[params.$stateParams.zoom] * 1000;
      var columnIndex = 0;

      angular.forEach(statDesc.blocks, function (block, index) {
        block.withTotal = block.columns && block.columns[block.columns.length - 1] === "Total";
        angular.forEach(block.stats, function (info) {
          var sample = samples[info.name];
          statsByName[info.name] = info;
          if (block.columns) {
            info.column = block.columns[columnIndex];
            if (columnIndex === (block.columns.length - 1)) {
              columnIndex = 0;
            } else {
              columnIndex ++;
            }
          } else {
            info.column = null;
          }
          info.config = {
            data: sample || [],
            breakInterval: breakInterval,
            timeOffset: timeOffset,
            now: stats.clientDate,
            zoomMillis: zoomMillis,
            timestamp: sample && sample.timestamps || stats.stats[stats.mainStatsBlock].timestamp,
            maxY: info.maxY,
            isBytes: info.isBytes,
            value: !sample ? 'N/A' : mnFormatQuantityFilter(sample[sample.length - 1], info.isBytes ? 1024 : 1000)
          };
        });
      });

      rv.isSpecificStats = !!params.$stateParams.specificStat;
      rv.specificStat = params.$stateParams.specificStat;

      rv.statsByName = statsByName;

      // Alpha sort the UI only Eventing Stats
      //section in Server/Statistics to prevent real-time reorders
      statDesc.blocks.sort(function (a,b) {
        if (a.blockName.startsWith("Eventing Stats:") &&
            b.blockName.startsWith("Eventing Stats:")) {
          if (a.blockName < b.blockName) {
            return -1;
          }
          if (a.blockName > b.blockName) {
            return 1
          }
        }
        return 0;
      });

      rv.statsDirectoryBlocks = statDesc.blocks;
      rv.stats = stats;
      rv.origTitle = data[1].origTitle;

      return rv;
    }
  }
})();
