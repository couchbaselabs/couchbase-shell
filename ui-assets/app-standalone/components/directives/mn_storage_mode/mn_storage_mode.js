(function () {
  "use strict";

  angular
    .module('mnStorageMode', [])
    .directive('mnStorageMode', mnStorageModeDirective)
    .filter('mnFormatStorageModeError', mnFormatStorageModeError);

  function mnFormatStorageModeError() {
    return function (error) {
      if (!error) {
        return;
      }
      var errorCode =
          error.indexOf("Storage mode cannot be set to") > -1 ? 1 :
          error.indexOf("storageMode must be one of") > -1 ? 2 :
          0;
      switch (errorCode) {
      case 1:
        return "please choose another index storage mode";
      case 2:
        return "please choose an index storage mode";
      default:
        return error;
      }
    };
  }

   function mnStorageModeDirective() {
    var mnStorageMode = {
      restrict: 'E',
      scope: {
        mnIsEnterprise: "=",
        mnModel: "=",
        mnErrors: "=",
        mnCompat: "=?",
        mnPermissions: "=?",
        mnServicesModel: "=?",
        mnInitial: "=?"
      },
      templateUrl: 'app/components/directives/mn_storage_mode/mn_storage_mode.html'
    };

    return mnStorageMode;
  }
})();
