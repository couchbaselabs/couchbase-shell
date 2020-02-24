(function () {
  "use strict";

  angular
    .module("mnViewsListService", [
      'mnTasksDetails',
      'mnBucketsService',
    ])
    .factory("mnViewsListService", mnViewsListFactory);

  function mnViewsListFactory($http, $q, mnTasksDetails, mnBucketsService) {
    var mnViewsListService = {
      createDdoc: createDdoc,
      getDdocUrl: getDdocUrl,
      getDdoc: getDdoc,
      deleteDdoc: deleteDdoc,
      cutOffDesignPrefix: cutOffDesignPrefix,
      getDdocs: getDdocs,
      getViewsListState: getViewsListState,
      getDdocsByType: getDdocsByType,
      getTasksOfCurrentBucket: getTasksOfCurrentBucket,
      isDevModeDoc: isDevModeDoc
    };

    return mnViewsListService;

    function handleCouchRequest(resp) {
      var data = {
        json : resp.data,
        meta : JSON.parse(resp.headers("X-Couchbase-Meta"))
      };
      return data;
    }
    function createDdoc(url, json) {
      return $http({
        method: 'PUT',
        url: url,
        data: json,
        mnHttp: {
          isNotForm: true
        }
      }).then(handleCouchRequest);
    }
    function getDdocUrl(bucket, name) {
      var encodedName = encodeURIComponent(cutOffDesignPrefix(name));
      if (name.indexOf("_design/dev_") > -1) {
        encodedName = "_design/dev_" + encodedName;
      } else if (name.indexOf("_design/") > -1) {
        encodedName = "_design/" + encodedName;
      }
      return '/couchBase/' + encodeURIComponent(bucket) + '/' + encodedName;
    }
    function getDdoc(url) {
      return $http({
        method: 'GET',
        url: url
      }).then(handleCouchRequest);
    }
    function deleteDdoc(url) {
      return $http({
        method: 'DELETE',
        url: url
      });
    }
    function cutOffDesignPrefix(id) {
      return id.replace(/^_design\/(dev_|)/, "");
    }
    function getDdocs(bucket, mnHttpParams) {
      return $http({
        method: "GET",
        url: '/pools/default/buckets/' + encodeURIComponent(bucket) + '/ddocs',
        mnHttp: mnHttpParams
      });
    }
    function isDevModeDoc(id) {
      var devPrefix = "_design/dev_";
      return id.substring(0, devPrefix.length) == devPrefix;
    }

    function getDdocsByType(bucket) {
      return getDdocs(bucket).then(function (resp) {
        var ddocs = resp.data;
        ddocs.development = _.filter(ddocs.rows, function (row) {
          return isDevModeDoc(row.doc.meta.id);
        });
        ddocs.production = _.reject(ddocs.rows, function (row) {
          return isDevModeDoc(row.doc.meta.id);
        });
        return ddocs;
      }, function (resp) {
        switch (resp.status) {
        case 0:
        case -1: return $q.reject(resp);
        case 404: return !bucket ? {status: "_404"} : resp;
        default: return resp;
        }
      });
    }

    function getTasksOfCurrentBucket(params) {
      return mnTasksDetails.get().then(function (tasks) {
        var rv = {};
        var importance = {
          view_compaction: 2,
          indexer: 1
        };

        _.each(tasks.tasks, function (taskInfo) {
          if ((taskInfo.type !== 'indexer' && taskInfo.type !== 'view_compaction') || taskInfo.bucket !== params.bucket) {
            return;
          }
          var ddoc = taskInfo.designDocument;
          (rv[ddoc] || (rv[ddoc] = [])).push(taskInfo);
        });
        _.each(rv, function (ddocTasks, key) {
          ddocTasks.sort(function (taskA, taskB) {
            return importance[taskA.type] - importance[taskB.type];
          });
        });

        return rv;
      });
    }
    function getViewsListState(params) {
      return getDdocsByType(params.bucket);
    }
  }
})();
