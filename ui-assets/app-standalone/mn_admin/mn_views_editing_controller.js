(function () {
  "use strict";

  angular
    .module("mnViews")
    .controller("mnViewsEditingController", mnViewsEditingController);

  function mnViewsEditingController($scope, $state, $uibModal, mnHelper, mnViewsEditingService, mnViewsListService, mnPromiseHelper) {
    var vm = this;
    var codemirrorOptions = {
      lineNumbers: true,
      lineWrapping: true,
      matchBrackets: true,
      tabSize: 2,
      mode: {
        name: "javascript",
        json: true
      },
      theme: 'default',
      readOnly: false
    }
    var viewsOptions = _.clone(codemirrorOptions);
    var sampleDocumentOptions = _.clone(codemirrorOptions);
    sampleDocumentOptions.readOnly = true;
    sampleDocumentOptions.lineWrapping = true;
    var sampleMetaOptions = _.clone(sampleDocumentOptions);

    vm.currentBucketName = $state.params.bucket;
    vm.viewsOptions = viewsOptions;
    vm.sampleDocumentOptions = sampleDocumentOptions;
    vm.sampleMetaOptions = sampleMetaOptions;
    vm.isSpatial = $state.params.isSpatial;
    vm.viewId = $state.params.viewId;
    vm.previewRandomDocument = previewRandomDocument;
    vm.awaitingSampleDocument = awaitingSampleDocument;
    vm.onReduceChange = onReduceChange;
    vm.setReduceValue = setReduceValue;
    vm.awaitingViews = awaitingViews;
    vm.goToDocumentsSection = goToDocumentsSection;
    vm.isEditDocumentDisabled = isEditDocumentDisabled;
    vm.toggleSampleDocument = toggleSampleDocument;
    vm.isViewsEditorControllsDisabled = isViewsEditorControllsDisabled;
    vm.isPreviewRandomDisabled = isPreviewRandomDisabled;
    vm.onSelectViewName = onSelectViewName;
    vm.toggleViews = toggleViews;
    vm.saveAs = saveAs;
    vm.save = save;
    vm.isFilterOpened = false;

    activate();

    function goToDocumentsSection(e) {
      e.stopImmediatePropagation();
      $state.go("app.admin.buckets.documents.editing", {
        documentId: vm.state.sampleDocument.meta.id,
        bucket: $state.params.bucket
      });
    }
    function toggleSampleDocument() {
      vm.isSampleDocumentClosed = !vm.isSampleDocumentClosed;
    }
    function toggleViews() {
      vm.isViewsClosed = !vm.isViewsClosed;
    }
    function hasNoWritePermission() {
      return !$scope.rbac.cluster.bucket[$state.params.bucket].views.write;
    }
    function isEditDocumentDisabled() {
      return awaitingSampleDocument() || (vm.state.sampleDocument && vm.state.sampleDocument.warnings) || vm.state.isEmptyState || hasNoWritePermission();
    }
    function isPreviewRandomDisabled() {
      return awaitingSampleDocument() || vm.state.isEmptyState || hasNoWritePermission();
    }
    function isViewsEditorControllsDisabled() {
      return awaitingViews() || vm.state.isEmptyState || !vm.state.isDevelopmentDocument || hasNoWritePermission();
    }
    function awaitingSampleDocument() {
      return !vm.state || vm.state.sampleDocumentLoading
    }
    function awaitingViews() {
      return !vm.state || vm.state.viewsLoading;
    }
    function isViewPathTheSame(current, selected) {
      return current.viewId === selected.viewId && current.isSpatial === selected.isSpatial && current.documentId === selected.documentId;
    }
    function previewRandomDocument(e) {
      e && e.stopImmediatePropagation && e.stopImmediatePropagation();
      mnPromiseHelper(vm.state, mnViewsEditingService.prepareRandomDocument($state.params))
        .showSpinner("sampleDocumentLoading")
        .applyToScope("sampleDocument");
    }
    function saveAs(e) {
      e.stopImmediatePropagation();
      $uibModal.open({
        controller: 'mnViewsCreateDialogController as viewsCreateDialogCtl',
        templateUrl: 'app/mn_admin/mn_views_create_dialog.html',
        scope: $scope,
        resolve: {
          currentDdoc: mnHelper.wrapInFunction(vm.state.currentDocument.doc),
          viewType: mnHelper.wrapInFunction($state.params.isSpatial ? "spatial" : "views")
        }
      }).result.then(function (vm) {
        var selected = {
          documentId: '_design/dev_' + vm.ddoc.name,
          isSpatial: vm.isSpatial,
          viewId: vm.ddoc.view
        };
        if (!isViewPathTheSame($state.params, selected)) {
          $state.go('^.result', {
            viewId: selected.viewId,
            documentId: selected.documentId
          });
        }
      });
    }
    function setReduceValue(value) {
      if (isViewsEditorControllsDisabled()) {
        return;
      }
      vm.state.currentDocument.doc.json.views[vm.viewId].reduce = value;
    }
    function onReduceChange(view) {
      if (view.reduce === "") {
        delete view.reduce;
      }
    }
    function save(e) {
      e.stopImmediatePropagation();
      var url = mnViewsListService.getDdocUrl($state.params.bucket, vm.state.currentDocument.doc.meta.id)
      mnPromiseHelper(vm.state, mnViewsListService.createDdoc(url, vm.state.currentDocument.doc.json))
        .catchErrors("viewsError")
        .showSpinner("viewsLoading")
        .showGlobalSuccess("View saved successfully!");
    }
    function onSelectViewName(selected) {
      $state.go('^.result', {
        viewId: selected.viewId,
        isSpatial: selected.isSpatial,
        documentId: selected.documentId
      });
    }

    function activate() {
      $scope.$watch(isViewsEditorControllsDisabled, function (isDisabled) {
        viewsOptions.readOnly = !!isDisabled;
        viewsOptions.matchBrackets = !isDisabled;
        vm.viewsOptions = viewsOptions;
      });
      return mnPromiseHelper(vm, mnViewsEditingService.getViewsEditingState($state.params))
        .applyToScope("state");
    }
  }
})();
