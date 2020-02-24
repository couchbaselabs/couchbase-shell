(function() {
  'use strict';

    angular.module('qwQuery',
                   ["ui.router",
                    "mnJquery",
                    'qwJsonTree',
                    'qwJsonDataTable',
                    'qwExplainVizD3',
                    'qwLongPress',
                    'mnBucketsService',
                    'mnHelper',
                    'mnPendingQueryKeeper',
                    'mnServersService',
                    'mnPoolDefault',
                    'mnAuth',
                    'mnPools',
                    'mnElementCrane',
                    'ui.ace',
                    'ui.bootstrap',
                    'ng-showdown'])

        .config(function($stateProvider,$urlRouterProvider) {
        $urlRouterProvider.otherwise('/standalone/workbench');

      $stateProvider
      .state('app', {
        abstract: true,
        controller: 'qwQueryController',
        url: '/standalone',
        template: '<ui-view/>'
      })
      .state('app.workbench', {
        controller: 'qwQueryController',
        url: '/workbench',
        templateUrl: 'ui-current/query.html'
      })
      ;

    })

    //

    // we can only work if we have a query node. This service checks for
    // a query node a reports back whether it is present.

        .factory('validateQueryService', function($http,mnServersService,mnPermissions, mnPoolDefault, mnPools) {
            mnPools.get().then(function() {_isEnterprise = mnPools.export.isEnterprise;});
            mnPoolDefault.get();
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
        var queryData = {statement: "select raw keyspaces.name from system:keyspaces;"};
        $http.post("/_p/query/query/service",queryData)
        .then(function success(resp) {
          //var data = resp.data, status = resp.status;
          //console.log("Got bucket list data: " + JSON.stringify(resp).substring(0,10) + " with callbacks: " + _callbackList.length);
          mnPermissions.check().then(function success() {
            updateValidBuckets(resp.data.results);
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

      function updateValidBuckets(allBuckets) {
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
             if (k != '.') {
              _bucketList.push(k);
               _bucketStatsList.push(k);
            }
             else
               _bucketList = allBuckets;
          });

          //console.log("valid bucketList: " + JSON.stringify(_bucketList));
        //console.log("bucketStatsList: " + JSON.stringify(_bucketStatsList));

        // all done
        _valid = true; _bucketsInProgress = false;
      }


      // now return the service
      return service;
    });


    angular.module('app', ['ui.router','mnPools','mnElementCrane']).run(appRun);

  // can't get authentication running right now.
    function appRun($state,$urlRouter,mnPools) {
        console.log("App run...");
        mnPools.get();
    mnPools.get().then(function (pools) {
      console.log("Pools: " + pools.isInitialized);
      if (!pools.isInitialized) {
        console.log("Error, pool not initialized!");
//        return $state.go('app.workbench');
//        //return $state.go('app.wizard.welcome');
      }
    }, function (resp) {
      console.log("Got response: " + JSON.stringify(resp));
//
//      switch (resp.status) {
//        case 401:
//          console.log("Going to app.auth");
//          return $state.go('app.auth', null, {location: false});
//      }
    }).then(function () {
      console.log(".then...");
//      $urlRouter.listen();
//      $urlRouter.sync();
    });
  }


})();
