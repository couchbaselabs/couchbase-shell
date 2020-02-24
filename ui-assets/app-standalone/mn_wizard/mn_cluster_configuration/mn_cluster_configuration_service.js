(function () {
  "use strict";

  angular.module('mnClusterConfigurationService', [
    'mnHelper',
    'mnPools'
  ]).factory('mnClusterConfigurationService', mnClusterConfigurationServiceFactory);

  function mnClusterConfigurationServiceFactory($http, mnHelper, IEC, mnPools) {
    var mnClusterConfigurationService = {
      getDynamicRamQuota: getDynamicRamQuota,
      getJoinClusterConfig: getJoinClusterConfig,
      getNewClusterConfig: getNewClusterConfig,
      getSelfConfig: getSelfConfig,
      postDiskStorage: postDiskStorage,
      postHostname: postHostname,
      postJoinCluster: postJoinCluster,
      lookup: lookup,
      getConfig: getConfig,
      postAuth: postAuth,
      postStats: postStats,
      getQuerySettings: getQuerySettings,
      postQuerySettings: postQuerySettings,
      postCurlWhitelist: postCurlWhitelist,
      postEnableExternalListener: postEnableExternalListener,
      postSetupNetConfig: postSetupNetConfig
    };
    var re = /^[A-Z]:\//;
    var preprocessPath;

    var joinClusterConfig = {
      clusterMember: {
        hostname: "127.0.0.1",
        username: "Administrator",
        password: ''
      },
      services: {
        disabled: {kv: false, index: false, n1ql: false, fts: false},
        model: {kv: true, index: true, n1ql: true, fts: true}
      },
      firstTimeAddedServices: undefined
    };
    var newConfig = {
      maxMemorySize: undefined,
      totalMemorySize: undefined,
      memoryQuota: undefined,
      displayedServices: {kv: true, index: true, fts: true, n1ql: true},
      services: {
        disabled: {kv: true, index: false, n1ql: false, fts: false},
        model: {kv: true, index: true, n1ql: true, fts: true}
      },
      showKVMemoryQuota: true,
      showIndexMemoryQuota: true,
      showFTSMemoryQuota: true,
      indexMemoryQuota: undefined,
      ftsMemoryQuota: undefined,
      eventingMemoryQuota: undefined,
      cbasMemoryQuota: undefined,
      minMemorySize: 256,
      indexSettings: {
        storageMode: mnPools.export.isEnterprise ? "plasma" : "forestdb"
      }
    };
    if (mnPools.export.isEnterprise) {
      newConfig.displayedServices.cbas = true;
      newConfig.services.disabled.cbas = false;
      newConfig.services.model.cbas = true;
      joinClusterConfig.services.disabled.cbas = false;
      joinClusterConfig.services.model.cbas = true;
      newConfig.showCBASMemoryQuota = true;

      newConfig.displayedServices.eventing = true;
      newConfig.services.disabled.eventing = false;
      newConfig.services.model.eventing = true;
      joinClusterConfig.services.disabled.eventing = false;
      joinClusterConfig.services.model.eventing = true;
      newConfig.showEventingMemoryQuota = true;

    }

    return mnClusterConfigurationService;

    function postEnableExternalListener(data) {
      return $http({
        method: 'POST',
        url: '/node/controller/enableExternalListener',
        data: data
      });
    }

    function postSetupNetConfig(data) {
      return $http({
        method: 'POST',
        url: '/node/controller/setupNetConfig',
        data: data
      });
    }

    function postQuerySettings(data) {
      return $http({
        method: 'POST',
        url: '/settings/querySettings',
        data: data
      }).then(function (resp) {
        return resp.data;
      });
    }

    function getQuerySettings() {
      return $http({
        method: 'GET',
        url: '/settings/querySettings'
      }).then(function (resp) {
        return resp.data;
      });
    }

    function postCurlWhitelist(data, initData) {
      data = _.clone(data);
      if (data.all_access && initData) {
        data.allowed_urls = initData.allowed_urls;
        data.disallowed_urls = initData.disallowed_urls;
      }
      if (data.allowed_urls.length == 1 && data.allowed_urls[0] === "") {
        delete data.allowed_urls;
      }
      if (data.disallowed_urls.length == 1 && data.disallowed_urls[0] === "") {
        delete data.disallowed_urls;
      }
      return $http({
        method: 'POST',
        mnHttp: {
          isNotForm: true
        },
        url: '/settings/querySettings/curlWhitelist',
        data: data
      }).then(function (resp) {
        return resp.data;
      });
    }

    function postStats(sendStats) {
      return doPostStats({sendStats: sendStats});
    }

    function doPostStats(data) {
      return $http({
        method: 'POST',
        url: '/settings/stats',
        data: data
      });
    }

    function postAuth(user, justValidate) {
      var data = _.clone(user);
      delete data.verifyPassword;
      data.port = "SAME";

      return $http({
        method: 'POST',
        url: '/settings/web',
        data: data,
        params: {
          just_validate: justValidate ? 1 : 0
        }
      });
    }

    function getConfig() {
      return mnClusterConfigurationService.getSelfConfig().then(function (resp) {
        var selfConfig = resp;
        var rv = {};
        rv.selfConfig = selfConfig;

        newConfig.maxMemorySize = selfConfig.ramMaxMegs;
        newConfig.totalMemorySize = selfConfig.ramTotalSize;
        newConfig.memoryQuota = selfConfig.memoryQuota;
        newConfig.indexMemoryQuota = selfConfig.indexMemoryQuota;
        newConfig.ftsMemoryQuota = selfConfig.ftsMemoryQuota;
        newConfig.eventingMemoryQuota = selfConfig.eventingMemoryQuota;
        newConfig.calculateTotal = true;

        if (mnPools.export.isEnterprise) {
          newConfig.cbasMemoryQuota = selfConfig.cbasMemoryQuota;
          rv.cbasDirs = selfConfig.storage.hdd[0].cbas_dirs;
        }

        rv.startNewClusterConfig = newConfig;
        rv.hostname = selfConfig.hostname;
        rv.dbPath = selfConfig.storage.hdd[0].path;
        rv.indexPath = selfConfig.storage.hdd[0].index_path;
        rv.eventingPath = selfConfig.storage.hdd[0].eventing_path;
        rv.java_home = selfConfig.storage.hdd[0].java_home;
        rv.addressFamily = selfConfig.addressFamily;
        rv.nodeEncryption = selfConfig.nodeEncryption;

        return rv;
      });
    }
    function getDynamicRamQuota() {
      return newConfig.memoryQuota;
    }
    function getNewClusterConfig() {
      return newConfig;
    }

    function preprocessPathStandard(p) {
      if (p.charAt(p.length-1) != '/') {
        p += '/';
      }
      return p;
    }
    function preprocessPathForWindows(p) {
      p = p.replace(/\\/g, '/');
      if (re.exec(p)) { // if we're using uppercase drive letter downcase it
        p = String.fromCharCode(p.charCodeAt(0) + 0x20) + p.slice(1);
      }
      return preprocessPathStandard(p);
    }
    function updateTotal(pathResource) {
      return (Math.floor(pathResource.sizeKBytes * (100 - pathResource.usagePercent) / 100 / IEC.Mi)) + ' GB';
    }
    function getJoinClusterConfig() {
      return joinClusterConfig;
    }
    function getSelfConfig() {
      return $http({
        method: 'GET',
        url: '/nodes/self'
      }).then(function (resp) {
        var nodeConfig = resp.data;
        var ram = nodeConfig.storageTotals.ram;
        var totalRAMMegs = Math.floor(ram.total / IEC.Mi);
        preprocessPath = (nodeConfig.os === 'windows' || nodeConfig.os === 'win64' || nodeConfig.os === 'win32') ? preprocessPathForWindows : preprocessPathStandard;
        nodeConfig.preprocessedAvailableStorage = _.map(_.clone(nodeConfig.availableStorage.hdd, true), function (storage) {
          storage.path = preprocessPath(storage.path);
          return storage;
        }).sort(function (a, b) {
          return b.path.length - a.path.length;
        });

        nodeConfig.ramTotalSize = totalRAMMegs;
        nodeConfig.ramMaxMegs = mnHelper.calculateMaxMemorySize(totalRAMMegs);

        nodeConfig.hostname = (nodeConfig && nodeConfig['otpNode'].split('@')[1]) || '127.0.0.1';
        if (nodeConfig.hostname == "cb.local") {
            nodeConfig.hostname = "127.0.0.1";
        }

        return nodeConfig;
      });
    }
    function lookup(path, availableStorage) {
      return updateTotal(path && _.detect(availableStorage, function (info) {
        return preprocessPath(path).substring(0, info.path.length) == info.path;
      }) || {path: "/", sizeKBytes: 0, usagePercent: 0});
    }
    function postDiskStorage(config, node) {
      return $http({
        method: 'POST',
        url: '/nodes/' + (node || 'self') + '/controller/settings',
        data: config
      });
    }
    function postHostname(hostname) {
      return $http({
        method: 'POST',
        url: '/node/controller/rename',
        data: {hostname: hostname}
      });
    }
    function postJoinCluster(clusterMember) {
      clusterMember.user = clusterMember.username;
      return $http({
        method: 'POST',
        url: '/node/controller/doJoinCluster',
        data: clusterMember
      });
    }
  }
})();
