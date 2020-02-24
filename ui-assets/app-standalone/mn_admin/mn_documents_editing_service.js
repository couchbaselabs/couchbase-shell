(function () {
  "use strict";

  angular
    .module("mnDocumentsEditingService", ["mnBucketsService", "mnFilters"])
    .factory("mnDocumentsEditingService", mnDocumentsEditingFactory);

  function mnDocumentsEditingFactory($http, mnBucketsService, $q, getStringBytesFilter, docBytesLimit) {
    var mnDocumentsEditingService = {
      getDocument: getDocument,
      createDocument: createDocument,
      deleteDocument: deleteDocument,
      getDocumentsEditingState: getDocumentsEditingState,
      isJsonOverLimited: isJsonOverLimited
    };

    return mnDocumentsEditingService;

    function isJsonOverLimited(json) {
      return getStringBytesFilter(json) > docBytesLimit;
    }

    function getDocumentsEditingState(params) {
      return getDocument(params).then(function getDocumentState(resp) {
        var doc = resp.data
        var rv = {};
        var editorWarnings = {
          documentIsBase64: ("base64" in doc),
          documentLimitError: isJsonOverLimited(doc.json)
        };
        rv.title = doc.meta.id;
        if (_.chain(editorWarnings).values().some().value()) {
          rv.editorWarnings = editorWarnings;
        } else {
          rv.doc = js_beautify(doc.json, {"indent_size": 2});
          rv.meta = JSON.stringify(doc.meta, null, "  ");
        }
        return rv;
      }, function (resp) {
        switch (resp.status) {
          case 404: return {
            editorWarnings: {
              notFound: true
            },
            title: params.documentId
          };
          default: return {
            errors: resp && resp.data,
          };
        }
      });
    }

    function deleteDocument(params) {
      return $http({
        method: "DELETE",
        url: buildDocumentUrl(params)
      });
    }

    function createDocument(params, doc, flags) {
      return $http({
        method: "POST",
        url: buildDocumentUrl(params),
        data: {
          flags: flags || 0x02000006,
          value: js_beautify(doc, {
            "indent_size": 0,
            "eol": "",
            "remove_space_before_token": true,
            "indent_char": ""}) || '{"click": "to edit", "with JSON": "there are no reserved field names"}'
        }
      });
    }

    function getDocument(params) {
      if (!params.documentId) {
        return $q.reject({data: {reason: "Document ID cannot be empty"}});
      }
      return $http({
        method: "GET",
        url: buildDocumentUrl(params)
      });
    }
    function buildDocumentUrl(params) {
      return "/pools/default/buckets/" + encodeURIComponent(params.bucket) + "/docs/" + encodeURIComponent(params.documentId);
    }
  }
})();
