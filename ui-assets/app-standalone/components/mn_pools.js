(function () {
  "use strict";

  angular
    .module('mnPools', [
    ])
    .factory('mnPools', mnPoolsFactory);

  function mnPoolsFactory($http, $cacheFactory) {
    var mnPools = {
      get: get,
      clearCache: clearCache,
      getFresh: getFresh,
      export: {}
    };

    var launchID =  (new Date()).valueOf() + '-' + ((Math.random() * 65536) >> 0);

    return mnPools;

    function get(mnHttpParams) {
      return $http({
        method: 'GET',
        url: '/pools',
        cache: true,
        mnHttp: mnHttpParams,
        requestType: 'json'
      }).then(function (resp) {
        var pools = resp.data;
        pools.isInitialized = !!pools.pools.length;
        pools.launchID = pools.uuid + '-' + launchID;
        mnPools.export.isEnterprise = pools.isEnterprise;
        return pools;
      });
    }
    function clearCache() {
      $cacheFactory.get('$http').remove('/pools');
      return this;
    }
    function getFresh() {
      return mnPools.clearCache().get();
    }
  }
})();
