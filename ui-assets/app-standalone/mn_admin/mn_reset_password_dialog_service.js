(function () {
  "use strict";

  angular
    .module("mnResetPasswordDialogService", [
      "ui.bootstrap"
    ])
    .factory("mnResetPasswordDialogService", mnResetPasswordDialogFactory);

  function mnResetPasswordDialogFactory($http, $uibModal, $q) {
    var mnResetPasswordDialogService = {
      post: post,
      showDialog: showDialog
    };

    return mnResetPasswordDialogService;

    function showDialog(user) {
      $uibModal.open({
        templateUrl: 'app/mn_admin/mn_reset_password_dialog.html',
        controller: "mnResetPasswordDialogController as resetPasswordDialogCtl",
        resolve: {
          user: function () {
            return user;
          }
        }
      });
    }

    function post(user) {
      return $http({
        headers: {
          'Authorization': "Basic " + btoa(user.name + ":" + user.currentPassword),
          'ns-server-ui': undefined
        },
        url: "/controller/changePassword",
        method: "POST",
        data: {
          password: user.password
        }
      }).then(function (resp) {
        return resp.data;
      }, function (resp) {
        if (resp.status === 401) {
          return $q.reject("Incorrect user password");
        } else {
          return $q.reject(resp);
        }
      });
    }
  }
})();
