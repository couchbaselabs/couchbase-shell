(function () {
  "use strict";

  angular
    .module('mnSessionService', [
      "mnAuthService",
      "mnPoolDefault",
      'ui.bootstrap',
    ])
    .factory('mnSessionService', mnSessionFactory);

  function mnSessionFactory($http, $window, $timeout, mnAuthService, mnPoolDefault, $uibModal, $interval) {
    var mnSession = {
      post: post,
      get: get,
      init: init,
      setTimeout: setTimeout,
      resetTimeoutAndSyncAcrossTabs: resetTimeoutAndSyncAcrossTabs,
      showTimeoutDialog: showTimeoutDialog
    };
    var throttledResetTimeoutAndSyncAcrossTabs = _.throttle(resetTimeoutAndSyncAcrossTabs, 300);
    var sessionTimer;
    var sessionTimeoutDialog;
    var showTimeoutDialogTimer;

    return mnSession;

    function init($scope) {
      angular.element($window).on("storage", doSyncAcrossTabs);
      angular.element($window).on("mousemove keydown touchstart",
                                  throttledResetTimeoutAndSyncAcrossTabs);

      $scope.$on("$destroy", function () {
        angular.element($window).off("mousemove keydown touchstart",
                                     throttledResetTimeoutAndSyncAcrossTabs);
        angular.element($window).off("storage", doSyncAcrossTabs);
      });
    }

    function post(uiSessionTimeout) {
      return $http({
        method: "POST",
        url: "/settings/security",
        data: {
          uiSessionTimeout: uiSessionTimeout ? (uiSessionTimeout * 60) : undefined
        }
      });
    }

    function get() {
      return mnPoolDefault.get().then(function (resp) {
        return {
          uiSessionTimeout: (Number(resp.uiSessionTimeout) / 60) || 0
        };
      });
    }

    function showTimeoutDialog(timeout) {
      return function () {
        sessionTimeoutDialog = $uibModal.open({
          controller: function ($scope) {
            var timer = $interval(function () {
              --$scope.time;
            }, 1000);
            $scope.time = (timeout / 1000);
            $scope.$on("$destroy", function () {
              $interval.cancel(timer);
            });
          },
          templateUrl: 'app/mn_admin/mn_session_timeout_dialog.html'
        });

        sessionTimeoutDialog.result.then(function () {
          sessionTimeoutDialog = null;
          resetTimeoutAndSyncAcrossTabs(); //closed by user
        }, function (closedBy) {
          sessionTimeoutDialog = null;
          if (!closedBy) {
            resetTimeoutAndSyncAcrossTabs(); //dismissed by user
          }
        });
      }
    }

    function resetTimeout(timeout) {
      timeout = Number(timeout);
      var dialogTimeout;
      if (!!sessionTimer) {
        $timeout.cancel(sessionTimer);
      }
      if (!!showTimeoutDialogTimer) {
        $timeout.cancel(showTimeoutDialogTimer);
      }
      if (!!timeout) {
        dialogTimeout = timeout - 30000;
        sessionTimer = $timeout(mnAuthService.logout.bind(mnAuthService), timeout);
        showTimeoutDialogTimer = $timeout(showTimeoutDialog(dialogTimeout), dialogTimeout);
      }
    }

    function setTimeout(uiSessionTimeout) {
      localStorage.setItem("uiSessionTimeout", Number(uiSessionTimeout) * 1000);
    }

    function resetTimeoutAndSyncAcrossTabs() {
      if (sessionTimeoutDialog) {
        return;
      }
      resetTimeout(localStorage.getItem("uiSessionTimeout"));
      localStorage.setItem("mnResetSessionTimeout",
                           Number(localStorage.getItem("mnResetSessionTimeout") || "0") + 1);
    }

    function doSyncAcrossTabs(e) {
      if (e.key === "mnResetSessionTimeout") {
        if (sessionTimeoutDialog) {
          sessionTimeoutDialog.dismiss("reset");
        }
        resetTimeout(localStorage.getItem("uiSessionTimeout"));
      }
    }
  }
})();
