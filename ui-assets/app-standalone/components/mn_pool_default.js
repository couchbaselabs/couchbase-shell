(function () {
  "use strict";

  angular
    .module('mnPoolDefault', [
      'mnPools',
      'ui.bootstrap'
    ])
    .factory('mnPoolDefault', mnPoolDefaultFactory);

  function mnPoolDefaultFactory($http, $q, mnPools, $window, $location, $httpParamSerializerJQLike, mnHelper, $state) {
    var latest = {};
    var mnPoolDefault = {
      latestValue: latestValue,
      get: get,
      clearCache: clearCache,
      getFresh: getFresh,
      setHideNavSidebar: setHideNavSidebar,
      getUrlsRunningService: getUrlsRunningService,
      export: {
        compat: undefined,
        hideNavSidebar: false
      }
    };

    var version50 = encodeCompatVersion(5, 0);
    var version51 = encodeCompatVersion(5, 1);
    var version55 = encodeCompatVersion(5, 5);
    var version65 = encodeCompatVersion(6, 5);
    var cache;
    var request;

    return mnPoolDefault;

    function setHideNavSidebar(value) {
      mnPoolDefault.export.hideNavSidebar = value;
    }

    function latestValue() {
      return latest;
    }
    // counterpart of ns_heart:effective_cluster_compat_version/0
    function encodeCompatVersion(major, minor) {
      if (major < 2) {
        return 1;
      }
      return major * 0x10000 + minor;
    }
    function get(params, mnHttpParams) {
      if (!(params && params.etag) && cache) {
        return $q.when(cache);
      }
      if (request && !cache) {
        return request;
      }
      params = params || {waitChange: 0};
      request = $q.all([
        $http({
          mnHttp: mnHttpParams,
          method: 'GET',
          url: '/pools/default',
          params: params,
          timeout: 30000
        }),
        mnPools.get(mnHttpParams)
      ]).then(function (resp) {
        var poolDefault = resp[0].data;
        var pools = resp[1]
        poolDefault.rebalancing = poolDefault.rebalanceStatus !== 'none';
        //TODO replace serverGroupsUri in isGroupsAvailable using mixed cluster version
        poolDefault.isGroupsAvailable = !!(pools.isEnterprise && poolDefault.serverGroupsUri);
        poolDefault.isEnterprise = pools.isEnterprise;
        poolDefault.thisNode = _.detect(poolDefault.nodes, function (n) {
          return n.thisNode;
        });
        poolDefault.compat = {
          atLeast51: poolDefault.thisNode.clusterCompatibility >= version51,
          atLeast55: poolDefault.thisNode.clusterCompatibility >= version55,
          atLeast65: poolDefault.thisNode.clusterCompatibility >= version65
        };
        poolDefault.versions = {
          "50": version50,
          "51": version51,
          "55": version55,
          "65": version65
        };
        poolDefault.capiBase = $window.location.protocol === "https:" ? poolDefault.thisNode.couchApiBaseHTTPS : poolDefault.thisNode.couchApiBase;

        _.extend(mnPoolDefault.export, poolDefault);
        latest.value = poolDefault; //deprecated and superseded by mnPoolDefault.export
        cache = poolDefault;

        return poolDefault;
      }, function (resp) {
        if ((resp.status === 404 && resp.data === "unknown pool") || resp.status === 500) {
          mnHelper.reloadApp();
        }
        return $q.reject(resp);
      });
      return request;
    }
    function clearCache() {
      cache = undefined;
      request = undefined;
      return this;
    }
    function getFresh(params) {
      return mnPoolDefault.clearCache().get(params);
    }
    /**
     * getUrlsRunningService - returns a list of URLs for nodes in the cluster
     *   running the named service. It assumes that you are currently on a page
     *   associated with the service, and it appends the path for the current page
     *   to the URL.
     *
     * @param nodeInfos - details on the nodes in the cluster returned
     *                    by
     * @param service - name of service
     * @param max - optional max number of links to return
     *
     * @return a list of URLs for the current UI location running the
     *         specified service.
     */
    function parseHostname(href) {
      var l = document.createElement("a");
      l.href = href;
      return l;
    }

    function getUrlsRunningService(nodeInfos, service, max) {
      var nodes = _.filter(nodeInfos, function (node) {
        return _.indexOf(node.services, service) > -1
          && node.clusterMembership === 'active';
      });
      if (max && max < nodes.length) {
        nodes = nodes.slice(0, max);
      }
      var protocol = $location.protocol();
      var search = $httpParamSerializerJQLike($location.search());
      var hash = $location.hash();
      var ext = $state.transition ? parseHostname(
                                      $state.href(
                                        $state.transition.to().name,
                                        $state.transition.params("to"),
                                        {absolute: true}
                                      )
                                    ).hash
            : "#!" + $location.path() +
              (search ? "?" + search : "") +
              (hash ? "#" + hash : "");

      return _.map(nodes, function(node) {
        // ipv4/ipv6/hostname + port
        var link = parseHostname("http://" + node.hostname);
        var port = protocol == "https" ? node.ports.httpsMgmt : link.port;
        return protocol
          + "://" + link.hostname
          + ":" + port
          + $window.location.pathname
          + ext;
      });
    }
  }
})();
