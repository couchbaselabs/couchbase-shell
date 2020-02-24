(function () {
  "use strict";

  angular
    .module("mnDocumentsListService", ["mnBucketsService"])
    .factory("mnDocumentsListService", mnDocumentsListFactory);

  function mnDocumentsListFactory($http, $q, mnBucketsService, docsLimit) {
    var mnDocumentsListService = {
      getDocuments: getDocuments,
      getDocumentsListState: getDocumentsListState,
      getDocumentsParams: getDocumentsParams,
      getDocumentsURI: getDocumentsURI
    };

    return mnDocumentsListService;

    function getListState(docs, params) {
      var rv = {};
      rv.pageNumber = params.pageNumber;
      rv.isNextDisabled = docs.rows.length <= params.pageLimit || params.pageLimit * (params.pageNumber + 1) === docsLimit;
      if (docs.rows.length > params.pageLimit) {
        docs.rows.pop();
      }

      rv.docs = docs;

      rv.pageLimits = [10, 20, 50, 100];
      rv.pageLimits.selected = params.pageLimit;
      return rv;
    }

    function getDocumentsListState(params) {
      return getDocuments(params).then(function (resp) {
        return getListState(resp.data, params);
      }, function (resp) {
        switch (resp.status) {
        case 0:
        case -1: return $q.reject(resp);
        case 404: return !params.bucket ? {status: "_404"} : resp;
        case 501: return {};
        default: return resp;
        }
      });
    }

    function getDocumentsParams(params) {
      var param;
      try {
        param = JSON.parse(params.documentsFilter) || {};
      } catch (e) {
        param = {};
      }
      var page = params.pageNumber;
      var limit = params.pageLimit;
      var skip = page * limit;

      param.skip = String(skip);
      param.include_docs = true;
      param.limit = String(limit + 1);

      if (param.startkey) {
        param.startkey = JSON.stringify(param.startkey);
      }

      if (param.endkey) {
        param.endkey = JSON.stringify(param.endkey);
      }
      return param;
    }

    function getDocumentsURI(params) {
      return "/pools/default/buckets/" + encodeURIComponent(params.bucket) + "/docs";
    }

    function getDocuments(params) {
      return $http({
        method: "GET",
        url: getDocumentsURI(params),
        params: getDocumentsParams(params)
      });
    }
  }
})();
