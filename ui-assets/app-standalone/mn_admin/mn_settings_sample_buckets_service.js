(function () {
  "use strict";

  angular
    .module("mnSettingsSampleBucketsService", [
      "mnPoolDefault",
      "mnTasksDetails",
      "mnBucketsService",
      "mnServersService"
    ])
    .factory("mnSettingsSampleBucketsService", mnSettingsSampleBucketsFactory);

  function mnSettingsSampleBucketsFactory($http, $q, mnPoolDefault, mnTasksDetails, mnBucketsService, mnServersService) {
    var mnSettingsSampleBucketsService = {
      getSampleBuckets: getSampleBuckets,
      installSampleBuckets: installSampleBuckets,
      getSampleBucketsState: getSampleBucketsState
    };

    return mnSettingsSampleBucketsService;

    function getSampleBucketsState(selectedBuckets) {
      return $q.all([
        getSampleBuckets(),
        mnPoolDefault.getFresh(),
        mnTasksDetails.get(),
        mnBucketsService.getBucketsByType(),
        mnServersService.getNodes()
      ]).then(function (resp) {
        var warnings = {
          quota: false,
          rebalance: false,
          maxBucketCount: false
        };
        var attentions = {};
        var sampleBuckets = resp[0];
        var poolDefault = resp[1];
        var tasks = resp[2];
        var buckets = resp[3];
        var servers = resp[4];

        var numServers = servers.active.length;
        var quotaAvailable = poolDefault.storageTotals.ram.quotaTotal - poolDefault.storageTotals.ram.quotaUsed;
        var maxNumBuckets = poolDefault.maxBucketCount;
        var numExistingBuckets = buckets.length;

        var storageNeeded = _.reduce(selectedBuckets, function (acc, quotaNeeded) {
          return acc + parseInt(quotaNeeded, 10);
        }, 0) * numServers;

        if (!(storageNeeded <= quotaAvailable)) {
          warnings.quota = Math.ceil(storageNeeded - quotaAvailable) / 1024 / 1024 / numServers;
        }
        warnings.maxBucketCount = (numExistingBuckets + _.keys(selectedBuckets).length > maxNumBuckets) && maxNumBuckets;
        warnings.rebalance = tasks.inRebalance;

        attentions.noIndexOrQuery = !_.find(servers.active, function (server) {
          return _.indexOf(server.services, "index") > -1;
        }) || !_.find(servers.active, function (server) {
          return _.indexOf(server.services, "n1ql") > -1;
        });

        return {
          installed: _.filter(sampleBuckets, 'installed', true),
          available: _.filter(sampleBuckets, 'installed', false),
          warnings: warnings,
          attentions: attentions
        };
      });
    }
    function getSampleBuckets() {
      return $http({
        url: '/sampleBuckets',
        method: 'GET'
      }).then(function (resp) {
        return resp.data;
      });
    }
    function installSampleBuckets(selectedSamples) {
      return $http({
        url: '/sampleBuckets/install',
        method: 'POST',
        timeout: 140000,
        data: JSON.stringify(_.keys(_.pick(selectedSamples, _.identity)))
      });
    }
  }
})();
