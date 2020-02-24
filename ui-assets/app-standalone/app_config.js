(function () {
  "use strict";

  angular
    .module('app')
    .config(appConfig);

  function appConfig($httpProvider, $stateProvider, $urlRouterProvider, $uibModalProvider, $transitionsProvider, $uibTooltipProvider, $animateProvider, $qProvider, $sceDelegateProvider) {
    $httpProvider.defaults.headers.common['invalid-auth-response'] = 'on';
    $httpProvider.defaults.headers.common['Cache-Control'] = 'no-cache';
    $httpProvider.defaults.headers.common['Pragma'] = 'no-cache';
    $httpProvider.defaults.headers.common['ns-server-ui'] = 'yes';

    $animateProvider.classNameFilter(/enable-ng-animation/);

    $sceDelegateProvider.resourceUrlWhitelist([
      'self', // Allow same origin resource loads
      'https://ph.couchbase.net/**' // Allow JSONP calls that match this pattern
    ]);

    $qProvider.errorOnUnhandledRejections(false);
    // When using a tooltip in an absolute positioned element,
    // you need tooltip-append-to-body="true" https://github.com/angular-ui/bootstrap/issues/4195
    $uibTooltipProvider.options({
      placement: "auto right",
      trigger: "outsideClick"
    });

    $urlRouterProvider.rule(function ($injector, $location) {
      //If hashprefix entered incorrectly, angular redirects to
      //url with correct hashprefix and inserts everything
      //after it using encodeURIcompoment (e.g #/asdasd -> #!#%2Fasdasd)
      //With this rule, we insert orinal hash right after correct
      //hashprefix (e.g. #!/asdasd)
      if ($location.url().indexOf("#") === 0) {
        return $location.hash();
      }
    });

    $urlRouterProvider.otherwise(function ($injector, $location) {
      $injector.get("mnPools").get().then(function (pools) {
        if (pools.isInitialized) {
          return $injector.get("$state").go("app.admin.overview");
        }
      });
      return true;
    });

    $stateProvider.state('app', {
      url: '?{enableInternalSettings:bool}&{disablePoorMansAlerts:bool}',
      params: {
        enableInternalSettings: {
          value: null,
          squash: true
        },
        disablePoorMansAlerts: {
          value: null,
          squash: true
        }
      },
      abstract: true,
      resolve: {
        env: function (mnEnv, $rootScope) {
          return mnEnv.loadEnv().then(function(env) {
            $rootScope.ENV = env;
          });
        }
      },
      template: '<div ui-view="" class="root-container"></div>' +
        '<div ng-show="mnGlobalSpinnerFlag" class="global-spinner"></div>'
    });

    $transitionsProvider.onStart({
      from: "app.admin.**",
      to: "app.admin.**"
    }, function (trans) {
      var mnLostConnectionService = trans.injector().get('mnLostConnectionService');
      var state = mnLostConnectionService.getState();
      //block navigation in app.admin while lostConnection is active
      //and allow navigation for the moment of reloading (e.g location.reload)
      return !state.isActivated || state.isReloading;
    });

    $transitionsProvider.onFinish({
      from: "app.admin.**",
      to: "app.admin.**"
    }, function (trans) {
      var $rootScope = trans.injector().get('$rootScope');
      $rootScope.showMainSpinner = false;
    });

    $transitionsProvider.onError({
      from: "app.admin.**",
      to: "app.admin.**"
    }, function (trans) {
      var $rootScope = trans.injector().get('$rootScope');
      $rootScope.showMainSpinner = false;
    });

    $transitionsProvider.onBefore({
      from: "app.admin.**",
      to: "app.admin.**"
    }, function (trans) {
      var mnPools = trans.injector().get('mnPools');
      var $rootScope = trans.injector().get('$rootScope');
      var mnPendingQueryKeeper = trans.injector().get('mnPendingQueryKeeper');
      var $uibModalStack = trans.injector().get('$uibModalStack');
      var isModalOpen = !!$uibModalStack.getTop();
      var toName = trans.to().name;
      var fromName = trans.from().name;
      if ($rootScope.mnGlobalSpinnerFlag) {
        return false;
      }
      if (!isModalOpen && toName.indexOf(fromName) === -1 && fromName.indexOf(toName) === -1) {
        //cancel tabs specific queries in case toName is not child of fromName and vise versa
        mnPendingQueryKeeper.cancelTabsSpecificQueries();
        $rootScope.showMainSpinner = true;
      }
      return !isModalOpen;
    });
    $transitionsProvider.onBefore({
      from: "app.auth",
      to: "app.admin.**"
    }, function (trans, $state) {
      var mnPools = trans.injector().get('mnPools');
      return mnPools.get().then(function (pools) {
        return pools.isInitialized ? true : $state.target("app.wizard.welcome");
      }, function (resp) {
        switch (resp.status) {
          case 401: return false;
        }
      });
    });
    $transitionsProvider.onBefore({
      from: "app.wizard.**",
      to: "app.admin.**"
    }, function (trans) {
      var mnPools = trans.injector().get('mnPools');
      return mnPools.getFresh().then(function (pools) {
        return pools.isInitialized;
      });
    });
    $transitionsProvider.onBefore({
      from: "app.wizard.**",
      to: "app.auth"
    }, function (trans) {
      var mnPools = trans.injector().get('mnPools');
      return mnPools.get().then(function (pools) {
        return pools.isInitialized;
      }, function (resp) {
        switch (resp.status) {
        case 401: return true;
        }
      });
    });
    $transitionsProvider.onBefore({
      from: "app.admin.**",
      to: "app.auth"
    }, function (trans) {
      var mnPools = trans.injector().get('mnPools');
      return mnPools.get().then(function () {
        return false;
      }, function (resp) {
        switch (resp.status) {
          case 401: return true;
        }
      });
    });
    $transitionsProvider.onStart({
      to: function (state) {
        return state.data && state.data.permissions;
      }
    }, function (trans) {
      var mnPermissions = trans.injector().get('mnPermissions');
      var $parse = trans.injector().get('$parse');
      return mnPermissions.check().then(function() {
        return !!$parse(trans.to().data.permissions)(mnPermissions.export);
      });
    });
    $transitionsProvider.onStart({
      to: function (state) {
        return state.data && state.data.compat;
      }
    }, function (trans) {
      var mnPoolDefault = trans.injector().get('mnPoolDefault');
      var $parse = trans.injector().get('$parse');
      return mnPoolDefault.get().then(function() {
        return !!$parse(trans.to().data.compat)(mnPoolDefault.export.compat);
      });
    });
    $transitionsProvider.onStart({
      to: function (state) {
        return state.data && state.data.ldap;
      }
    }, function (trans) {
      var mnPoolDefault = trans.injector().get('mnPoolDefault');
      return mnPoolDefault.get().then(function(value) {
        return value.saslauthdEnabled;
      });
    });
    $transitionsProvider.onStart({
      to: function (state) {
        return state.data && state.data.enterprise;
      }
    }, function (trans) {
      var mnPools = trans.injector().get('mnPools');
      return mnPools.get().then(function (pools) {
        return pools.isEnterprise;
      });
    });

    $urlRouterProvider.deferIntercept();
  }
})();
