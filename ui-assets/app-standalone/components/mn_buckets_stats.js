(function () {
  "use strict";

  angular
    .module("mnBucketsStats", [])
    .factory("mnBucketsStats", mnBucketsFactory);

  function mnBucketsFactory($http, $cacheFactory) {
    var mnBucketsStats = {
      get: get,
      clearCache: clearCache,
    };

    return mnBucketsStats;

    function get(mnHttpParams) {
      return $http({
        method: "GET",
        cache: true,
        url: '/pools/default/buckets?basic_stats=true&skipMap=true',
        mnHttp: mnHttpParams
      });
    }

    function clearCache() {
      $cacheFactory.get('$http').remove('/pools/default/buckets?basic_stats=true&skipMap=true');
      return this;
    }
  }
})();
