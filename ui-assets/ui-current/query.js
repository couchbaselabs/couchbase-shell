(function () {
  "use strict";

  angular
  .module('qwQuery', ["ui.router", "mnPluggableUiRegistry", "mnJquery",
    'qwJsonTree',
    'qwJsonDataTable',
    'qwJsonTableEditor',
    'qwExplainVizD3',
    'qwLongPress',
    'qwValidJson',
    'mnPendingQueryKeeper',
    'mnServersService',
    'mnPools',
    'mnPoolDefault',
    'mnPermissions',
    'ngclipboard',
    'ui.ace',
    'ui.bootstrap'])
    .config(function($stateProvider, $transitionsProvider, mnPluggableUiRegistryProvider, mnPermissionsProvider) {

      $stateProvider
      .state('app.admin.query', {
        abstract: true,
        url: '/query',
        views: {
          "main@app.admin": {
            controller: 'qwQueryController',
            templateUrl: '../_p/ui/query/ui-current/query_toplevel.html'
          }
        },
        data: {
          title: "Query"
        }
      });

      $stateProvider
      .state('app.admin.doc_editor', {
        url: '/doc_editor?bucket',
        views: {
          "main@app.admin": {
            controller: 'qwDocEditorController',
            templateUrl: '../_p/ui/query/ui-current/doc_editor.html'
          }
        },
        data: {
          title: "Documents"
        }
      });

      addQueryStates("app.admin.query");

      function addQueryStates(parent) {
        $stateProvider
        .state(parent + '.monitoring', {
          url: '/monitoring',
          controller: 'qwQueryMonitorController as qmc',
          templateUrl: '../_p/ui/query/ui-current/query_monitoring.html'
        })
        .state(parent + '.workbench', {
          url: '/workbench?query',
          controller: 'qwQueryController as qc',
          templateUrl: '../_p/ui/query/ui-current/query.html'
        })
        ;
      }

      mnPluggableUiRegistryProvider.registerConfig({
        name: 'Query',
        state: 'app.admin.query.workbench',
        includedByState: 'app.admin.query',
        plugIn: 'workbenchTab',
        index: 1
      });

      mnPluggableUiRegistryProvider.registerConfig({
        name: 'Documents',
        state: 'app.admin.doc_editor',
        includedByState: 'app.admin.doc_editor',
        plugIn: 'workbenchTab',
        ngShow: "rbac.cluster.bucket['.'].data.docs.read  && rbac.cluster.bucket['.'].data.xattr.read",
        index: 0
      });

      //
      // whenever the user logs out, we want ensure that validateQueryService knows it needs
      // to re-validate
      //

      $transitionsProvider.onFinish({
        from: "app.auth",
        to: "app.admin.**",
      }, function ($transition$, $state, $injector) {
        var injector = $injector || $transition$.injector();
        var qwQueryService = injector.get("qwQueryService");
        qwQueryService.updateBuckets();
        qwQueryService.loadStateFromStorage();
      });

      mnPermissionsProvider.set("cluster.n1ql.meta!read"); // system catalogs
      mnPermissionsProvider.setBucketSpecific(function (name) {
        return [
          "cluster.bucket[" + name + "].n1ql.select!execute",
          "cluster.bucket[" + name + "].data.docs!upsert",
          "cluster.bucket[" + name + "].data.xattr!read"
        ]
      })

    })
    .run(function(jQuery, $timeout, $http) {
    })

    // we can only work if we have a query node. This service checks for
    // a query node a reports back whether it is present.

    .factory('validateQueryService', function($http,mnServersService,mnPermissions, mnPools, mnPoolDefault) {
      mnPools.get().then(function() {_isEnterprise = mnPools.export.isEnterprise;});
      var _checked = false;              // have we checked validity yet?
      var _valid = false;                // do we have a valid query node?
      var _bucketsInProgress = false;    // are we retrieving the list of buckets?
      var _monitoringAllowed = false;
      var _clusterStatsAllowed = false;
      var _otherStatus;
      var _otherError;
      var _bucketList = [];
      var _bucketStatsList = [];
      var _callbackList = [];
      var _isEnterprise = false;
      var service = {
          inProgress: function()       {return !_checked || _bucketsInProgress;},
          isEnterprise: function()     {return(_isEnterprise);},
          valid: function()            {return _valid;},
          validBuckets: function()     {return _bucketList;},
          otherStatus: function()      {return _otherStatus;},
          otherError: function()       {return _otherError;},
          monitoringAllowed: function() {return _monitoringAllowed;},
          clusterStatsAllowed: function() {return _clusterStatsAllowed;},
          updateValidBuckets: getBuckets,
          getBucketsAndNodes: getBuckets
      }

      //
      // with RBAC the only safe way to get the list of buckets is through a query
      // of system:keyspaces, which should return only accessible buckets for the user.
      // we accept a callback function that will be called once the list of buckets is updated.
      //

      function getBuckets(callback) {
        //console.trace();
        //console.log("Getting nodes and buckets, progress: " + _bucketsInProgress);

        // even if we're busy, accept new callbacks
        if (callback)
          _callbackList.push(callback);

        // make sure we only do this once at a time
        if (_bucketsInProgress)
         return;

        //_valid = false;
        _checked = true;
        _otherStatus = null;
        _otherError = null;
        _bucketsInProgress = true;

        // meanwhile issue a query to the local node get the list of buckets
        var queryData = {statement: "select keyspaces.name from system:keyspaces;"};
        $http.post("/_p/query/query/service",queryData)
        .then(function success(resp) {
          //var data = resp.data, status = resp.status;
          //console.log("Got bucket list data: " + JSON.stringify(resp).substring(0,10) + " with callbacks: " + _callbackList.length);
          mnPermissions.check().then(function success() {
            updateValidBuckets();
            while (_callbackList.length) // call each callback to let them know we're done
              _callbackList.pop()();
          });
        },
        // Error from $http
        function error(resp) {
          var data = resp.data, status = resp.status;
          //console.log("Error getting buckets: " + JSON.stringify(resp));
          _valid = false; _bucketsInProgress = false;
          _otherStatus = status;
          _otherError = data;
          while (_callbackList.length) // call each callback to let them know we're done
            _callbackList.pop()();
        });
      }

      function updateValidBuckets() {
        // see what buckets we have permission to access
        var perms = mnPermissions.export.cluster;
        //console.log("Got bucket permissions... " + JSON.stringify(perms));

        _bucketList = []; _bucketStatsList = [];

        // stats perms
        _clusterStatsAllowed = (perms && perms.stats && perms.stats.read);

        // metadata perms
        _monitoringAllowed = (perms && perms.n1ql && perms.n1ql.meta && perms.n1ql.meta.read);

        // per-bucket perms
        if (perms && perms.bucket)
          _.forEach(perms.bucket,function(v,k) {
            // uncomment the following when RBAC is working properly for data access
            if (v && v.n1ql && v.n1ql.select && v.n1ql.select.execute)
              _bucketList.push(k);
            if (v && v.stats && v.stats.read && k != "*") {
              _bucketStatsList.push(k);
            }
          });

        //console.log("valid bucketList: " + JSON.stringify(_bucketList));
        //console.log("bucketStatsList: " + JSON.stringify(_bucketStatsList));

        // all done
        _valid = true; _bucketsInProgress = false;
      }


      // now return the service
      return service;
    });


  angular.module('mnAdmin').requires.push('qwQuery');
}());
