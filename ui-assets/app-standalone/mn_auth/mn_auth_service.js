(function () {
  "use strict";

  angular.module('mnAuthService', [
    'mnPools',
    'ui.router',
    'mnPendingQueryKeeper',
    'mnPermissions'
  ]).factory('mnAuthService', mnAuthServiceFactory);

  function mnAuthServiceFactory($http, $state, mnPools, $rootScope, mnPendingQueryKeeper, mnPermissions, $uibModalStack, $window, $q, $cacheFactory) {
    var mnAuthService = {
      login: login,
      logout: _.once(logout),
      whoami: whoami,
      canUseCertForAuth: canUseCertForAuth
    };

    return mnAuthService;

    function whoami() {
      return $http({
        method: 'GET',
        cache: true,
        url: '/whoami'
      }).then(function (resp) {
        return resp.data;
      });
    }

    function canUseCertForAuth() {
      return $http({
        method: 'GET',
        url: '/_ui/canUseCertForAuth'
      }).then(function (r) {
        return r.data;
      });
    }

    function login(user, useCertForAuth) {
      var config = {
        method: 'POST',
        url: '/uilogin'
      }

      if (useCertForAuth) {
        config.params = {
          use_cert_for_auth: 1
        };
      } else {
        user = user || {};
        config.data = {
          user: user.username,
          password: user.password
        };
      }

      return $http(config).then(function (resp) {
        return mnPools.get().then(function (cachedPools) {
          mnPools.clearCache();
          return mnPools.get().then(function (newPools) {
            if (cachedPools.implementationVersion !== newPools.implementationVersion) {
              return $q.reject({status: 410});
            } else {
              return resp;
            }
          });
        }).then(function (resp) {
          localStorage.setItem("mnLogIn",
                               Number(localStorage.getItem("mnLogIn") || "0") + 1);
          return resp;
        })
      });
    }
    function logout() {
      $uibModalStack.dismissAll("uilogout");
      return $http({
        method: 'POST',
        url: "/uilogout"
      }).then(function () {
        $window.location.reload();
      }, function () {
        $window.location.reload();
      });
    }
  }
})();
