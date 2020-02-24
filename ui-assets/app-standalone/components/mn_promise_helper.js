(function () {
  "use strict";

  angular
    .module('mnPromiseHelper', [
      'mnAlertsService',
      'mnHelper'
    ])
    .factory('mnPromiseHelper', mnPromiseHelperFactory);

  function mnPromiseHelperFactory(mnAlertsService, mnHelper, $timeout, $rootScope) {

    return mnPromiseHelper;

    function mnPromiseHelper(scope, promise, modalInstance) {
      var spinnerNameOrFunction = 'viewLoading';
      var errorsNameOrCallback = 'errors';
      var pendingGlobalSpinnerQueries = {};
      var spinnerTimeout;
      var promiseHelper = {
        applyToScope: applyToScope,
        getPromise: getPromise,
        onSuccess: onSuccess,
        reloadState: reloadState,
        closeFinally: closeFinally,
        closeOnSuccess: closeOnSuccess,
        showErrorsSensitiveSpinner: showErrorsSensitiveSpinner,
        catchErrorsFromSuccess: catchErrorsFromSuccess,
        showSpinner: showSpinner,
        showGlobalSpinner: showGlobalSpinner,
        catchErrors: catchErrors,
        catchGlobalErrors: catchGlobalErrors,
        showGlobalSuccess: showGlobalSuccess,
        broadcast: broadcast,
        removeErrors: removeErrors,
        closeModal: closeModal
      }

      return promiseHelper;

      function getPromise() {
        return promise;
      }
      function onSuccess(cb) {
        promise.then(cb);
        return this;
      }
      function reloadState(state) {
        promise.then(function () {
          spinnerCtrl(true);
          mnHelper.reloadState(state);
        });
        return this;
      }
      function closeFinally() {
        promise['finally'](closeModal);
        return this;
      }
      function closeOnSuccess() {
        promise.then(closeModal);
        return this;
      }
      function showErrorsSensitiveSpinner(name, timer, scope) {
        name && setSpinnerName(name);
        maybeHandleSpinnerWithTimer(timer, scope);
        promise.then(clearSpinnerTimeout, hideSpinner);
        return this;
      }
      function catchErrorsFromSuccess(nameOrCallback) {
        nameOrCallback && setErrorsNameOrCallback(nameOrCallback);
        promise.then(function (resp) {
          errorsCtrl(extractErrors(resp));
        });
        return this;
      }
      function showSpinner(name, timer, scope) {
        name && setSpinnerName(name);
        maybeHandleSpinnerWithTimer(timer, scope);
        promise.then(hideSpinner, hideSpinner);
        return this;
      }

      function showGlobalSpinner(timer) {
        var id = doShowGlobalSpinner();
        promise.then(hideGlobalSpinner(id), hideGlobalSpinner(id));
        return this;
      }
      function catchErrors(nameOrCallback) {
        nameOrCallback && setErrorsNameOrCallback(nameOrCallback);
        promise.then(removeErrors, function (resp) {
          if (resp.status !== -1) {
            errorsCtrl(extractErrors(resp));
          }
        });
        return this;
      }
      function catchGlobalErrors(errorMessage, timeout) {
        promise.then(null, function (resp) {
          if (resp.status !== -1) {
            mnAlertsService.formatAndSetAlerts(errorMessage || extractErrors(resp.data), 'error', timeout);
          }
        });
        return this;
      }
      function showGlobalSuccess(successMessage, timeout) {
        if (timeout === undefined) {
          timeout = 2500;
        }
        promise.then(function (resp) {
          mnAlertsService.formatAndSetAlerts(successMessage || resp.data, 'success', timeout);
        });
        return this;
      }
      function applyToScope(keyOrFunction) {
        promise.then(angular.isFunction(keyOrFunction) ? keyOrFunction : function (value) {
          scope[keyOrFunction] = value;
        }, function () {
          if (angular.isFunction(keyOrFunction)) {
            keyOrFunction(null);
          } else {
            delete scope[keyOrFunction];
          }
        });
        return this;
      }
      function broadcast(event, data) {
        promise.then(function () {
          $rootScope.$broadcast(event, data);
        });
        return this;
      }
      function spinnerCtrl(isLoaded) {
        if (angular.isFunction(spinnerNameOrFunction)) {
          spinnerNameOrFunction(isLoaded);
        } else {
          scope[spinnerNameOrFunction] = isLoaded;
        }
      }
      function errorsCtrl(errors) {
        if (angular.isFunction(errorsNameOrCallback)) {
          errorsNameOrCallback(errors);
        } else {
          scope[errorsNameOrCallback] = errors;
        }
      }
      function doShowGlobalSpinner() {
        var timer = $timeout(function () {
          $rootScope.mnGlobalSpinnerFlag = true;
        }, 100);
        var id = "id" + Math.random().toString(36).substr(2, 9);
        pendingGlobalSpinnerQueries[id] = timer;
        return id;
      }
      function hideGlobalSpinner(id) {
        return function () {
          $timeout.cancel(pendingGlobalSpinnerQueries[id]);
          delete pendingGlobalSpinnerQueries[id];
          if (_.isEmpty(pendingGlobalSpinnerQueries)) {
            $rootScope.mnGlobalSpinnerFlag = false;
          }
        }
      }
      function hideSpinner() {
        spinnerCtrl(false);
        clearSpinnerTimeout();
      }
      function removeErrors() {
        errorsCtrl(false);
        return this;
      }
      function setSpinnerName(name) {
        spinnerNameOrFunction = name;
      }
      function setErrorsNameOrCallback(nameOrCallback) {
        errorsNameOrCallback = nameOrCallback;
      }
      function closeModal() {
        modalInstance.close(scope);
      }
      function extractErrors(resp) {
        if (resp.status === 0) {
          return false;
        }
        var errors = resp.data && resp.data.errors !== undefined && _.keys(resp.data).length === 1 ? resp.data.errors : resp.data || resp ;
        return _.isEmpty(errors) ? false : errors;
      }
      function clearSpinnerTimeout() {
        if (spinnerTimeout) {
          $timeout.cancel(spinnerTimeout);
        }
      }
      function enableSpinnerTimeout(timer) {
        spinnerTimeout = $timeout(function () {
          spinnerCtrl(true);
        }, timer);
      }
      function maybeHandleSpinnerWithTimer(timer, scope) {
        if (timer) {
          enableSpinnerTimeout(timer);
          scope.$on("$destroy", clearSpinnerTimeout);
        } else {
          spinnerCtrl(true);
        }
      }
    }
  }
})();
