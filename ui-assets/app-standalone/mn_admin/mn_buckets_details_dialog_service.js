(function () {
  angular.module('mnBucketsDetailsDialogService', [
    'mnFilters',
    'mnPoolDefault',
    'mnServersService',
    'mnBucketsDetailsService',
    'mnSettingsAutoCompactionService',
    'mnAlertsService'
  ]).factory('mnBucketsDetailsDialogService', mnBucketsDetailsDialogServiceFactory);

  function mnBucketsDetailsDialogServiceFactory($http, $q, mnBytesToMBFilter, mnCountFilter, mnSettingsAutoCompactionService, mnPoolDefault, mnServersService, bucketsFormConfiguration, mnBucketsDetailsService) {
    var mnBucketsDetailsDialogService = {
      prepareBucketConfigForSaving: prepareBucketConfigForSaving,
      adaptValidationResult: adaptValidationResult,
      getNewBucketConf: getNewBucketConf,
      reviewBucketConf: reviewBucketConf,
      postBuckets: postBuckets
    };

    return mnBucketsDetailsDialogService;

    function postBuckets(data, uri) {
      return $http({
        data: data,
        method: 'POST',
        url: uri
      });
    }
    function prepareBucketConfigForSaving(bucketConf, autoCompactionSettings, poolDefault, pools) {
      var conf = {};
      function copyProperty(property) {
        if (bucketConf[property] !== undefined) {
          conf[property] = bucketConf[property];
        }
      }
      function copyProperties(properties) {
        properties.forEach(copyProperty);
      }
      if (bucketConf.isNew) {
        copyProperties(["name", "bucketType"]);
      }
      if (bucketConf.bucketType === "membase") {
        copyProperties(["autoCompactionDefined", "evictionPolicy"]);
        copyProperty("storageBackend");
      }
      if (bucketConf.bucketType === "ephemeral") {
        copyProperty("purgeInterval");
        conf["evictionPolicy"] = bucketConf["evictionPolicyEphemeral"];
      }
      if (bucketConf.bucketType === "membase" || bucketConf.bucketType === "ephemeral") {
        copyProperties(["threadsNumber", "replicaNumber"]);
        if (pools.isEnterprise && poolDefault.compat.atLeast55) {
          copyProperty("compressionMode");
          if (!bucketConf.enableMaxTTL) {
            conf.maxTTL = 0;
          } else {
            copyProperty("maxTTL");
          }
        }
        if (bucketConf.isNew) {
          if (bucketConf.bucketType !== "ephemeral") {
            copyProperty("replicaIndex");
          }

          if (pools.isEnterprise) {
            copyProperty("conflictResolutionType");
          }
        }

        if (bucketConf.autoCompactionDefined) {
          _.extend(conf, mnSettingsAutoCompactionService.prepareSettingsForSaving(autoCompactionSettings));
        }
      }

      if (bucketConf.isWizard) {
        copyProperty("otherBucketsRamQuotaMB");
      }

      copyProperties(["ramQuotaMB", "flushEnabled"]);

      return conf;
    }
    function adaptValidationResult(result) {
      var ramSummary = result.summaries.ramSummary;

      return {
        totalBucketSize: mnBytesToMBFilter(ramSummary.thisAlloc),
        nodeCount: mnCountFilter(ramSummary.nodesCount, 'node'),
        perNodeMegs: ramSummary.perNodeMegs,
        guageConfig: mnBucketsDetailsService.getBucketRamGuageConfig(ramSummary),
        errors: mnSettingsAutoCompactionService.prepareErrorsForView(result.errors)
      };
    }
    function getNewBucketConf() {
      return $q.all([
        mnServersService.getNodes(),
        mnPoolDefault.get()
      ]).then(function (resp) {
        var activeServersLength = resp[0].reallyActiveData.length;
        var totals = resp[1].storageTotals;
        var bucketConf = _.clone(bucketsFormConfiguration);
        bucketConf.isNew = true;
        bucketConf.ramQuotaMB = mnBytesToMBFilter(Math.floor((totals.ram.quotaTotal - totals.ram.quotaUsed) / activeServersLength));
        return bucketConf;
      });
    }
    function reviewBucketConf(bucketDetails) {
      return mnBucketsDetailsService.doGetDetails(bucketDetails).then(function (bucketConf) {
        bucketConf["evictionPolicyEphemeral"] = bucketConf["evictionPolicy"];
        bucketConf.ramQuotaMB = mnBytesToMBFilter(bucketConf.quota.rawRAM);
        bucketConf.threadsNumber = bucketConf.threadsNumber.toString();
        bucketConf.isDefault = bucketConf.name === 'default';
        bucketConf.enableMaxTTL = bucketConf.maxTTL !== 0;
        bucketConf.replicaIndex = bucketConf.replicaIndex ? 1 : 0;
        bucketConf.flushEnabled = (bucketConf.controllers !== undefined && bucketConf.controllers.flush !== undefined) ? 1 : 0;
        return bucketConf;
      });
    }
  }
})();
