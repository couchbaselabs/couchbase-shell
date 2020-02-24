(function () {
  "use strict";

  angular.module('app', [
    'mnAdmin',
    'mnAuth',
    'mnWizard',
    'mnHttp',
    'mnExceptionReporter',
    'ui.bootstrap',
    'mnEnv',
    'mnFilters'
  ]).run(appRun);

  function appRun($state, $urlRouter, $exceptionHandler, mnPools, $window, $rootScope, $location, $http, mnPrettyVersionFilter) {

    $rootScope.$on("$locationChangeStart", function (event, newUrl) {
      //angular do not replace url when it tries
      //to insert hashprefix in accordance with config (e.g. when user navigates
      //from url that starts with #!/ to #/). Such behaviour breaks back button.
      if ($location.url().indexOf("#") === 0) {
        $location.replace();
      }
    });

    angular.element($window).on("storage", function (storage) {
      if (storage.key === "mnLogIn") {
        $state.go("app.admin.overview");
      }
    });

    var originalOnerror = $window.onerror;
    $window.onerror = onError;
    function onError(message, url, lineNumber, columnNumber, exception) {
      $exceptionHandler({
        message: message,
        fileName: url,
        lineNumber: lineNumber,
        columnNumber: columnNumber,
        stack: exception && exception.stack
      });
      originalOnerror && originalOnerror.apply($window, Array.prototype.slice.call(arguments));
    }

    $http({method: "GET", url: "/versions"}).then(function (resp) {
      var pools = resp.data;
      var version = mnPrettyVersionFilter(pools.implementationVersion);
      $rootScope.mnTitle = "Couchbase Server";
    });

    mnPools.get().then(function (pools) {
      if (!pools.isInitialized) {
        return $state.go('app.wizard.welcome');
      }
    }, function (resp) {
      switch (resp.status) {
        case 401: return $state.go('app.auth', null, {location: false});
      }
    }).then(function () {
      $urlRouter.listen();
      $urlRouter.sync();
    });

    $state.defaultErrorHandler(function (error) {
      error && $exceptionHandler(error);
    });
  }
})();
