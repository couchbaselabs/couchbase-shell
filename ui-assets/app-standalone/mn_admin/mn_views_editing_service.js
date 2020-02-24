(function () {
  "use strict";

  angular
    .module("mnViewsEditingService", ["mnViewsListService", "mnDocumentsEditingService", "mnViewsListService", "mnFilters"])
    .factory("mnViewsEditingService", mnViewsEditingFactory);

  function mnViewsEditingFactory($http, $state, mnPermissions, mnViewsListService, mnDocumentsEditingService, mnDocumentsListService, $q, removeEmptyValueFilter, jQueryLikeParamSerializerFilter, mnPoolDefault, viewsPerPageLimit, docBytesLimit, getStringBytesFilter) {
    var mnViewsEditingService = {
      getViewsEditingState: getViewsEditingState,
      prepareRandomDocument: prepareRandomDocument,
      getViewResult: getViewResult,
      getFilterParams: getFilterParams,
      getFilterParamsAsString: getFilterParamsAsString,
      buildViewUrl: buildViewUrl,
      getInitialViewsFilterParams: getInitialViewsFilterParams
    };
    return mnViewsEditingService;

    function getInitialViewsFilterParams(isSpatial) {
      return isSpatial ? {
        limit: viewsPerPageLimit,
        stale: "false",
        connection_timeout: 60000,
      } : {
        limit: viewsPerPageLimit,
        stale: "false",
        connection_timeout: 60000,
        inclusive_end: true,
        reduce: ""
      };
    }

    function buildViewUrl(params) {
      if (!params || !params.documentId) {
        return;
      }
      var params = _.clone(params);
      if (params.documentId.slice(0, "_design/".length) === "_design/") {
        params.documentId = "_design/" + encodeURIComponent(params.documentId.slice("_design/".length));
      }
      if (params.documentId.slice(0, "_local/".length) === "_local/") {
        params.documentId = "_local/" + encodeURIComponent(params.documentId.slice("_local/".length));
      }
      return encodeURIComponent(params.bucket) + "/" + params.documentId + (params.isSpatial ? "/_spatial/" : "/_view/") + encodeURIComponent(params.viewId);
    }

    function getRandomKey(bucket) {
      return $http({
        method: "GET",
        url: "/pools/default/buckets/" + encodeURIComponent(bucket) + "/localRandomKey"
      });
    }
    function getFilterParamsAsString(params) {
      return "?" + jQueryLikeParamSerializerFilter(removeEmptyValueFilter(getFilterParams(params)));
    }
    function getFilterParams(params) {
      params = params || $state.params;
      var filterParams
      try {
        filterParams = JSON.parse(params.viewsParams) || {};
      } catch(e) {
        filterParams = {};
      }
      filterParams.skip = params.pageNumber * viewsPerPageLimit;
      filterParams.full_set = params.full_set;
      return filterParams;
    }
    function getViewResult(params) {
      return $http({
        method: "GET",
        url: "/couchBase/" + buildViewUrl(params),
        params: removeEmptyValueFilter(getFilterParams(params)),
        timeout: 3600000
      }).then(function (resp) {
        resp.data.rows = _.filter(resp.data.rows, function (r) {
          return ('key' in r);
        });
        return resp.data;
      }, function (view) {
        return mnViewsListService.getTasksOfCurrentBucket(params).then(function (tasks) {
          if (angular.isString(view.data)) {
            view = {
              data: {
                error: view.data
              }
            };
          }
          view.data.from = buildViewUrl(params) + getFilterParamsAsString(params);
          var indexingRunning = _.filter(tasks[params.documentId], function (item) {
            return item.type == "indexer" && item.status == "running" && item.designDocument == params.documentId;
          }).length;
          if ((view.data.error === "timeout" || (view.config && view.config.timeout.value === "timeout")) && indexingRunning) {
            view.data.error === "timeout";
            view.data.reason = "node is still building up the index";
            view.data.showBtn = true;
          }
          return $q.reject(view);
        });
      });
    }

    function prepareDocForCodeMirror(doc) {
      doc.metaJSON = angular.toJson(doc.meta, 2);
      doc.jsonJSON = js_beautify(doc.json || "", {"indent_size": 2});
      return doc;
    }

    function getSampleDocument(params) {
      return mnDocumentsEditingService.getDocument(params).then(function (sampleDocument) {
        if (getStringBytesFilter(angular.toJson(sampleDocument.data.json)) > docBytesLimit) {
          return {
            meta: sampleDocument.data.meta,
            warnings: {
              largeDocument: true
            }
          };
        }
        return prepareDocForCodeMirror(sampleDocument.data);
      }, function (resp) {
        switch(resp.status) {
          case 404: return {
            source: {
              meta: {
                id: resp.data.key
              }
            },
            warnings: {
              documentDoesNotExist: true
            }
          };
        }
      });
    }

    function prepareRandomDocument(params) {
      return params.sampleDocumentId ? getSampleDocument({
        documentId: params.sampleDocumentId,
        bucket: params.bucket
      }) : getRandomKey(params.bucket).then(function (resp) {
        return getSampleDocument({
          documentId: resp.data.key,
          bucket: params.bucket
        });
      }, function (resp) {
        switch(resp.status) {
          case 404:
            if (resp.data.error === "fallback_to_all_docs") {
              return mnDocumentsListService.getDocuments({
                bucket: params.bucket,
                pageNumber: 0,
                pageLimit: 1
              }).then(function (resp) {
                if (resp.data.rows[0]) {
                  return prepareDocForCodeMirror(resp.data.rows[0].doc);
                } else {
                  return {
                    warnings: {
                      thereAreNoDocs: true
                    }
                  };
                }
              });
            } else {
              return {
                warnings: {
                  thereAreNoDocs: true
                }
              };
            }
        }
      });
    }
    function getEmptyViewState(params) {
      return {isEmptyState: true};
    }

    function getViewsEditingState(params) {
      return $http({
        method: "HEAD",
        url: "/couchBase/" + buildViewUrl(params)
      }).then(function () {
        return $q.all([
          mnViewsListService.getDdocsByType(params.bucket),
          mnPoolDefault.get()
        ]).then(function (resp) {
          var rv = {};
          var poolDefault = resp[1];
          rv.ddocs = resp[0];
          rv.capiBase = poolDefault.capiBase;
          rv.isDevelopmentDocument = mnViewsListService.isDevModeDoc(params.documentId)
          if (rv.ddocs.rows.length) {
            rv.currentDocument = _.find(rv.ddocs.rows, function (row) {
              return row.doc.meta.id === params.documentId;
            });
            if (mnPermissions.export.cluster.bucket[params.bucket].data.docs.read) {
              return prepareRandomDocument(params).then(function (randomDoc) {
                rv.sampleDocument = randomDoc;
                return rv;
              });
            } else {
              return rv;
            }
          } else {
            return rv;
          }
        });
      }, function (resp) {
        switch (resp.status) {
          default: return getEmptyViewState(params);
        }
      });
    }
  }
})();
